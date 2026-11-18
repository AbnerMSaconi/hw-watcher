// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Slint floating window + lazy Control Panel + Tokio telemetry/multi-tray event loop.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use hw_watcher_core::Sample;
use slint::{ComponentHandle, ModelRc, PhysicalPosition, SharedString, VecModel};
use tokio::sync::mpsc;

use crate::app_commands::{handle_cmd, wire_window_callbacks};
use crate::config::PanelConfig;
use crate::i18n::{self, PoTranslator};
use crate::settings_ui::SettingsController;
use crate::source::{SourceMode, TelemetryClient};
use crate::tray::TrayCommand;
use crate::tray_slots::TrayCoordinator;
use crate::view_model::{PanelView, build_panel_view};
use crate::x11_window;

slint::include_modules!();

thread_local! {
    static SETTINGS: RefCell<SettingsController> = RefCell::new(SettingsController::default());
}

/// Shared state passed to the background collector thread.
pub struct CollectorCtx {
    pub weak: slint::Weak<Panel>,
    pub cfg: Arc<Mutex<PanelConfig>>,
    pub visible: Arc<AtomicBool>,
    pub skip_taskbar: Arc<AtomicBool>,
    pub blur_enabled: Arc<AtomicBool>,
    pub window_pending: Arc<AtomicBool>,
}

/// Launches the Slint floating panel, optional Control Panel, and SNI tray icon(s).
pub fn run_panel(start_in_tray: bool, open_settings: bool, mode: SourceMode) -> Result<()> {
    i18n::set_portuguese(i18n::detect_portuguese());
    let mut cfg_init = PanelConfig::load();
    if start_in_tray {
        cfg_init.visible = false;
    }
    if matches!(mode, SourceMode::Simulated { multi: true, .. }) {
        cfg_init.separate_cpus = true;
        cfg_init.separate_gpus = true;
    }
    x11_window::configure_backend(cfg_init.skip_taskbar);

    let panel = Panel::new().map_err(|e| anyhow::anyhow!("creating Slint window: {e}"))?;
    i18n::apply_to_slint();
    panel.set_keep_on_top(cfg_init.always_on_top);
    panel.set_panel_bg(cfg_init.theme.bg_slint());
    panel.set_panel_border(cfg_init.theme.border_slint());
    panel.set_muted_color(cfg_init.theme.muted_slint());
    panel.set_footer_color(cfg_init.theme.label_color("board"));
    panel
        .window()
        .set_position(PhysicalPosition::new(cfg_init.x, cfg_init.y));

    let ctx = CollectorCtx {
        weak: panel.as_weak(),
        cfg: Arc::new(Mutex::new(cfg_init.clone())),
        visible: Arc::new(AtomicBool::new(cfg_init.visible)),
        skip_taskbar: Arc::new(AtomicBool::new(cfg_init.skip_taskbar)),
        blur_enabled: Arc::new(AtomicBool::new(cfg_init.theme.blur_enabled)),
        window_pending: Arc::new(AtomicBool::new(
            cfg_init.visible && (cfg_init.skip_taskbar || cfg_init.theme.blur_enabled),
        )),
    };
    let (tx, rx) = mpsc::unbounded_channel::<TrayCommand>();

    x11_window::install_window_hooks(
        &panel,
        Arc::clone(&ctx.skip_taskbar),
        Arc::clone(&ctx.blur_enabled),
        Arc::clone(&ctx.window_pending),
    );
    wire_window_callbacks(&panel, &ctx.cfg, &tx);
    if open_settings {
        let _ = tx.send(TrayCommand::OpenSettings);
    }
    spawn_collector_thread(ctx, tx, rx, mode)?;

    if cfg_init.visible {
        panel
            .show()
            .map_err(|e| anyhow::anyhow!("showing Slint window: {e}"))?;
    }
    slint::run_event_loop_until_quit().map_err(|e| anyhow::anyhow!("Slint event loop failed: {e}"))
}

fn spawn_collector_thread(
    ctx: CollectorCtx,
    tx: mpsc::UnboundedSender<TrayCommand>,
    mut rx: mpsc::UnboundedReceiver<TrayCommand>,
    mode: SourceMode,
) -> Result<()> {
    std::thread::Builder::new()
        .name("hw-collector".into())
        .spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                tracing::error!("failed to create Tokio runtime");
                return;
            };
            rt.block_on(Box::pin(collector_loop(ctx, tx, &mut rx, mode)));
        })
        .context("spawning collector thread")?;
    Ok(())
}

async fn collector_loop(
    ctx: CollectorCtx,
    tx: mpsc::UnboundedSender<TrayCommand>,
    rx: &mut mpsc::UnboundedReceiver<TrayCommand>,
    mode: SourceMode,
) {
    let c0 = snap_cfg(&ctx.cfg);
    let mut tray_coord = TrayCoordinator::spawn(tx.clone(), &c0).await;
    let mut client = TelemetryClient::open(mode).await;
    let mut tick = make_interval(c0.interval_ms);
    let mut latest = client.next_sample().await;
    let mut prev_view: Option<PanelView> = None;
    refresh_frame(&ctx, &mut tray_coord, &latest, &c0, &mut prev_view).await;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                latest = client.next_sample().await;
                let cfg_snap = snap_cfg(&ctx.cfg);
                refresh_frame(&ctx, &mut tray_coord, &latest, &cfg_snap, &mut prev_view).await;
                let s_clone = latest.clone();
                let c_arc = Arc::clone(&ctx.cfg);
                let _ = slint::invoke_from_event_loop(move || {
                    SETTINGS.with(|s| s.borrow_mut().update_sample(&s_clone, &c_arc));
                });
            }
            Some(cmd) = rx.recv() => {
                if handle_cmd(cmd, &ctx, &tx, &mut tray_coord, &latest, &mut prev_view, &mut tick).await {
                    break;
                }
            }
        }
    }
}

async fn refresh_frame(
    ctx: &CollectorCtx,
    tray_coord: &mut TrayCoordinator,
    latest: &Sample,
    cfg: &PanelConfig,
    prev_view: &mut Option<PanelView>,
) {
    let tr = PoTranslator::active();
    tray_coord.sync_and_refresh(latest, cfg, &tr).await;
    if ctx.visible.load(Ordering::Relaxed) {
        push_ui_if_changed(&ctx.weak, latest, cfg, prev_view, tr);
    }
}

pub(crate) fn push_ui_if_changed(
    weak: &slint::Weak<Panel>,
    sample: &Sample,
    cfg: &PanelConfig,
    prev_view: &mut Option<PanelView>,
    tr: PoTranslator,
) {
    let view = build_panel_view(sample, cfg, &tr);
    if prev_view.as_ref() == Some(&view) {
        return;
    }
    *prev_view = Some(view.clone());
    let rows: Vec<MetricRow> = view
        .rows
        .into_iter()
        .map(|r| MetricRow {
            label: r.label.into(),
            label_color: r.label_color,
            col1: r.col1.into(),
            col1_color: r.col1_color,
            col2: r.col2.into(),
            col2_color: r.col2_color,
        })
        .collect();
    let footers: Vec<SharedString> = view.footer_lines.into_iter().map(Into::into).collect();
    let badge = SharedString::from(view.source_badge);
    let (bg, border, muted, footer) = (
        view.bg_color,
        view.border_color,
        view.muted_color,
        view.footer_color,
    );
    with_panel(weak, move |p| {
        p.set_panel_bg(bg);
        p.set_panel_border(border);
        p.set_muted_color(muted);
        p.set_footer_color(footer);
        p.set_source_badge(badge);
        p.set_rows(ModelRc::new(VecModel::from(rows)));
        p.set_footer_lines(ModelRc::new(VecModel::from(footers)));
    });
}

pub(crate) fn open_settings_window(
    cfg: Arc<Mutex<PanelConfig>>,
    latest: Sample,
    tx: mpsc::UnboundedSender<TrayCommand>,
) {
    let _ = slint::invoke_from_event_loop(move || {
        SETTINGS.with(|ctrl| ctrl.borrow_mut().open(&cfg, &latest, &tx));
    });
}

pub(crate) fn close_settings_window() {
    let _ = slint::invoke_from_event_loop(|| {
        SETTINGS.with(|ctrl| ctrl.borrow_mut().close());
    });
}

pub(crate) fn sync_open_settings(cfg: &PanelConfig) {
    let snap = cfg.clone();
    let _ = slint::invoke_from_event_loop(move || {
        SETTINGS.with(|ctrl| ctrl.borrow().sync_if_open(&snap));
    });
}

pub(crate) fn quit_ui() {
    let _ = slint::invoke_from_event_loop(|| {
        SETTINGS.with(|ctrl| ctrl.borrow_mut().close());
        let _ = slint::quit_event_loop();
    });
}

pub(crate) fn make_interval(ms: u64) -> tokio::time::Interval {
    let mut tick = tokio::time::interval(Duration::from_millis(ms));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick
}

pub(crate) fn snap_cfg(cfg: &Arc<Mutex<PanelConfig>>) -> PanelConfig {
    cfg.lock().map(|g| g.clone()).unwrap_or_default()
}

pub(crate) fn with_panel(weak: &slint::Weak<Panel>, f: impl FnOnce(Panel) + Send + 'static) {
    let w = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(p) = w.upgrade() {
            f(p);
        }
    });
}

pub(crate) fn update_cfg(
    cfg: &Arc<Mutex<PanelConfig>>,
    f: impl FnOnce(&mut PanelConfig),
) -> PanelConfig {
    if let Ok(mut guard) = cfg.lock() {
        f(&mut guard);
        guard.save();
        return guard.clone();
    }
    PanelConfig::default()
}
