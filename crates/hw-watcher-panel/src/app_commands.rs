// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Command dispatch for tray menu actions and live Control Panel config updates.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hw_watcher_core::Sample;
use slint::{ComponentHandle, PhysicalPosition};
use tokio::sync::mpsc;

use crate::app::{
    CollectorCtx, Panel, close_settings_window, make_interval, open_settings_window,
    push_ui_if_changed, quit_ui, sync_open_settings, update_cfg, with_panel,
};
use crate::config::PanelConfig;
use crate::i18n::PoTranslator;
use crate::tray::TrayCommand;
use crate::tray_slots::TrayCoordinator;
use crate::view_model::PanelView;
use crate::x11_window;

/// Wires Slint floating panel drag/drop and context-menu callbacks.
pub fn wire_window_callbacks(
    panel: &Panel,
    cfg: &Arc<Mutex<PanelConfig>>,
    tx: &mpsc::UnboundedSender<TrayCommand>,
) {
    let w = panel.as_weak();
    panel.on_drag(move |dx, dy| {
        if let Some(p) = w.upgrade() {
            if x11_window::is_wayland(&p) {
                x11_window::start_wayland_drag(&p);
                return;
            }
            let scale = p.window().scale_factor();
            let pos = p.window().position();
            p.window().set_position(PhysicalPosition::new(
                pos.x + (dx * scale) as i32,
                pos.y + (dy * scale) as i32,
            ));
        }
    });

    let (w, cfg_drop) = (panel.as_weak(), Arc::clone(cfg));
    panel.on_drop_window(move || {
        if let Some(p) = w.upgrade()
            && !x11_window::is_wayland(&p)
        {
            let pos = p.window().position();
            update_cfg(&cfg_drop, |c| (c.x, c.y) = (pos.x, pos.y));
        }
    });

    let t = tx.clone();
    panel.on_toggle_visible(move || {
        let _ = t.send(TrayCommand::ToggleVisible);
    });
    let (t, w) = (tx.clone(), panel.as_weak());
    panel.on_toggle_top(move || {
        if let Some(p) = w.upgrade() {
            let _ = t.send(TrayCommand::AlwaysOnTop(!p.get_keep_on_top()));
        }
    });
    let t = tx.clone();
    panel.on_open_settings(move || {
        let _ = t.send(TrayCommand::OpenSettings);
    });
    let t = tx.clone();
    panel.on_quit(move || {
        let _ = t.send(TrayCommand::Quit);
    });
}

/// Processes one [`TrayCommand`]; returns `true` when the collector loop should exit.
pub async fn handle_cmd(
    cmd: TrayCommand,
    ctx: &CollectorCtx,
    tx: &mpsc::UnboundedSender<TrayCommand>,
    tray_coord: &mut TrayCoordinator,
    latest: &Sample,
    prev_view: &mut Option<PanelView>,
    tick: &mut tokio::time::Interval,
) -> bool {
    let tr = PoTranslator::active();
    match cmd {
        TrayCommand::OpenSettings => {
            open_settings_window(Arc::clone(&ctx.cfg), latest.clone(), tx.clone());
            return false;
        }
        TrayCommand::CloseSettings => {
            close_settings_window();
            return false;
        }
        TrayCommand::ToggleVisible => {
            let show = !ctx.visible.load(Ordering::Relaxed);
            let cfg_snap = update_cfg(&ctx.cfg, |c| c.visible = show);
            apply_visibility(ctx, show);
            *prev_view = None;
            tick.reset_immediately();
            sync_open_settings(&cfg_snap);
            return false;
        }
        TrayCommand::AlwaysOnTop(top) => {
            let cfg_snap = update_cfg(&ctx.cfg, |c| c.always_on_top = top);
            with_panel(&ctx.weak, move |p| p.set_keep_on_top(top));
            tray_coord.sync_and_refresh(latest, &cfg_snap, &tr).await;
            sync_open_settings(&cfg_snap);
        }
        TrayCommand::SkipTaskbar(skip) => {
            ctx.skip_taskbar.store(skip, Ordering::Relaxed);
            let cfg_snap = update_cfg(&ctx.cfg, |c| c.skip_taskbar = skip);
            with_panel(&ctx.weak, move |p| x11_window::apply_skip_taskbar(&p, skip));
            tray_coord.sync_and_refresh(latest, &cfg_snap, &tr).await;
            sync_open_settings(&cfg_snap);
        }
        TrayCommand::SetTrayMetric(metric) => {
            let cfg_snap = update_cfg(&ctx.cfg, |c| {
                c.tray_metric = metric;
                if !c.tray_metrics.contains(&metric) {
                    c.tray_metrics.insert(0, metric);
                }
            });
            tray_coord.sync_and_refresh(latest, &cfg_snap, &tr).await;
            sync_open_settings(&cfg_snap);
        }
        TrayCommand::SetTooltipSections(sec) => {
            let cfg_snap = update_cfg(&ctx.cfg, |c| c.tooltip = sec);
            tray_coord.sync_and_refresh(latest, &cfg_snap, &tr).await;
            sync_open_settings(&cfg_snap);
        }
        TrayCommand::ApplyConfig(new_cfg) => {
            // The Control Panel mutates the shared config before sending this command, so
            // changes are detected against the state last applied to the window, not `ctx.cfg`.
            let mut next = *new_cfg;
            next.sanitize();
            let cfg_snap = update_cfg(&ctx.cfg, |c| *c = next.clone());
            if tick.period() != Duration::from_millis(cfg_snap.interval_ms) {
                *tick = make_interval(cfg_snap.interval_ms);
            }
            if ctx.visible.load(Ordering::Relaxed) != cfg_snap.visible {
                apply_visibility(ctx, cfg_snap.visible);
            }
            let top = cfg_snap.always_on_top;
            with_panel(&ctx.weak, move |p| {
                if p.get_keep_on_top() != top {
                    p.set_keep_on_top(top);
                }
            });
            let skip = cfg_snap.skip_taskbar;
            if ctx.skip_taskbar.swap(skip, Ordering::Relaxed) != skip {
                with_panel(&ctx.weak, move |p| x11_window::apply_skip_taskbar(&p, skip));
            }
            let blur = cfg_snap.theme.blur_enabled;
            if ctx.blur_enabled.swap(blur, Ordering::Relaxed) != blur {
                with_panel(&ctx.weak, move |p| {
                    x11_window::apply_blur_behind(&p, blur);
                    x11_window::refresh_compositor(&p);
                });
            }
            tray_coord.sync_and_refresh(latest, &cfg_snap, &tr).await;
            *prev_view = None;
            if ctx.visible.load(Ordering::Relaxed) {
                push_ui_if_changed(&ctx.weak, latest, &cfg_snap, prev_view, tr);
            }
        }
        TrayCommand::Quit => {
            quit_ui();
            return true;
        }
    }
    false
}

fn apply_visibility(ctx: &CollectorCtx, show: bool) {
    ctx.visible.store(show, Ordering::Relaxed);
    if show
        && (ctx.skip_taskbar.load(Ordering::Relaxed) || ctx.blur_enabled.load(Ordering::Relaxed))
    {
        ctx.window_pending.store(true, Ordering::Release);
    }
    with_panel(&ctx.weak, move |p| {
        let _ = if show { p.show() } else { p.hide() };
    });
}
