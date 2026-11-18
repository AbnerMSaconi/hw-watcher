// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Lazy Control Panel window management (`SettingsWindow`).

use std::sync::{Arc, Mutex};

use hw_watcher_core::Sample;
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::SettingsWindow;
use crate::config::PanelConfig;
use crate::settings_state::{
    apply_bool_setting, apply_int_setting, move_tray_metric, set_target_hex, toggle_tray_metric,
};
use crate::settings_sync::{sync_active_color, sync_hardware_summary, sync_window};
use crate::theme::{PALETTE_SWATCHES, color_with_opacity, format_hex_rgb};
use crate::tray::TrayCommand;

/// Owns the lazily created Slint [`SettingsWindow`] (dropped when closed to keep idle RAM 0).
pub struct SettingsController {
    win: Option<SettingsWindow>,
    latest_sample: Arc<Mutex<Sample>>,
}

impl Default for SettingsController {
    fn default() -> Self {
        Self {
            win: None,
            latest_sample: Arc::new(Mutex::new(Sample::default())),
        }
    }
}

impl SettingsController {
    /// Opens or focuses the Control Panel window.
    pub fn open(
        &mut self,
        cfg: &Arc<Mutex<PanelConfig>>,
        latest: &Sample,
        tx: &UnboundedSender<TrayCommand>,
    ) {
        if let Ok(mut s) = self.latest_sample.lock() {
            *s = latest.clone();
        }
        if let Some(ref w) = self.win {
            if let Ok(guard) = cfg.lock() {
                sync_window(w, &guard, latest);
            }
            let _ = w.show();
            return;
        }
        let Ok(w) = SettingsWindow::new() else {
            return;
        };
        init_palette(&w);
        if let Ok(guard) = cfg.lock() {
            sync_window(&w, &guard, latest);
        }
        wire_callbacks(
            &w,
            Arc::clone(cfg),
            tx.clone(),
            Arc::clone(&self.latest_sample),
        );
        let _ = w.show();
        self.win = Some(w);
    }

    /// Updates cached sample & refreshes hardware summary if window is open.
    pub fn update_sample(&mut self, sample: &Sample, cfg: &Arc<Mutex<PanelConfig>>) {
        if let Ok(mut s) = self.latest_sample.lock() {
            *s = sample.clone();
        }
        if let Some(ref w) = self.win
            && let Ok(guard) = cfg.lock()
        {
            sync_hardware_summary(w, &guard, sample);
        }
    }

    /// Refreshes open Settings window when config changes externally (e.g. from tray menu).
    pub fn sync_if_open(&self, cfg: &PanelConfig) {
        if let Some(ref w) = self.win
            && let Ok(s) = self.latest_sample.lock()
        {
            sync_window(w, cfg, &s);
        }
    }

    /// Closes and drops the Control Panel window, freeing its memory.
    pub fn close(&mut self) {
        if let Some(w) = self.win.take() {
            let _ = w.hide();
        }
    }
}

fn init_palette(w: &SettingsWindow) {
    let swatches: Vec<Color> = PALETTE_SWATCHES
        .iter()
        .map(|h| color_with_opacity(h, 100, [0xff, 0xff, 0xff]))
        .collect();
    let hexes: Vec<SharedString> = PALETTE_SWATCHES.iter().map(|&h| h.into()).collect();
    w.set_palette_swatches(ModelRc::new(VecModel::from(swatches)));
    w.set_palette_hexes(ModelRc::new(VecModel::from(hexes)));
}

#[derive(Clone)]
struct UiCtx {
    weak: slint::Weak<SettingsWindow>,
    cfg: Arc<Mutex<PanelConfig>>,
    tx: UnboundedSender<TrayCommand>,
    sample: Arc<Mutex<Sample>>,
}

impl UiCtx {
    fn mutate(&self, f: impl FnOnce(&mut PanelConfig)) {
        if let Ok(mut guard) = self.cfg.lock() {
            f(&mut guard);
            let snap = guard.clone();
            drop(guard);
            if let Some(win) = self.weak.upgrade()
                && let Ok(smp) = self.sample.lock()
            {
                sync_window(&win, &snap, &smp);
            }
            let _ = self.tx.send(TrayCommand::ApplyConfig(Box::new(snap)));
        }
    }
}

fn wire_callbacks(
    win: &SettingsWindow,
    cfg: Arc<Mutex<PanelConfig>>,
    tx: UnboundedSender<TrayCommand>,
    sample_rc: Arc<Mutex<Sample>>,
) {
    let base = UiCtx {
        weak: win.as_weak(),
        cfg,
        tx: tx.clone(),
        sample: sample_rc,
    };

    let ctx = base.clone();
    win.on_set_bool(move |key, val| ctx.mutate(|g| apply_bool_setting(g, key.as_str(), val)));

    let ctx = base.clone();
    win.on_set_int(move |key, val| ctx.mutate(|g| apply_int_setting(g, key.as_str(), val)));

    let ctx = base.clone();
    win.on_toggle_tray_metric(move |key| ctx.mutate(|g| toggle_tray_metric(g, key.as_str())));

    let ctx = base.clone();
    win.on_move_tray_metric(move |key, delta| {
        ctx.mutate(|g| move_tray_metric(g, key.as_str(), delta));
    });

    let ctx = base.clone();
    win.on_select_color_target(move |target, label| {
        if let Some(w_ref) = ctx.weak.upgrade()
            && let Ok(guard) = ctx.cfg.lock()
        {
            w_ref.set_active_color_target(target.clone());
            w_ref.set_active_color_label(label);
            sync_active_color(&w_ref, &guard, target.as_str());
        }
    });

    let ctx = base.clone();
    win.on_set_active_rgb(move |red, green, blue| {
        let Some(w_ref) = ctx.weak.upgrade() else {
            return;
        };
        let target = w_ref.get_active_color_target().to_string();
        let hex = format_hex_rgb([
            red.clamp(0, 255) as u8,
            green.clamp(0, 255) as u8,
            blue.clamp(0, 255) as u8,
        ]);
        ctx.mutate(|g| set_target_hex(g, &target, &hex));
    });

    let ctx = base.clone();
    win.on_set_active_hex(move |hex| {
        let Some(w_ref) = ctx.weak.upgrade() else {
            return;
        };
        let target = w_ref.get_active_color_target().to_string();
        ctx.mutate(|g| set_target_hex(g, &target, hex.as_str()));
    });

    let ctx = base.clone();
    win.on_clear_item_color(move |key| {
        ctx.mutate(|g| {
            g.theme.item_label_colors.remove(key.as_str());
        });
    });

    let ctx = base.clone();
    win.on_apply_preset(move |preset| ctx.mutate(|g| g.theme.apply_preset(preset.as_str())));

    let ctx = base;
    win.on_reset_defaults(move || {
        ctx.mutate(|g| {
            let (pos_x, pos_y) = (g.x, g.y);
            *g = PanelConfig {
                x: pos_x,
                y: pos_y,
                ..Default::default()
            };
        });
    });

    let tx_close = tx.clone();
    win.on_close_settings(move || {
        let _ = tx_close.send(TrayCommand::CloseSettings);
    });
    win.window().on_close_requested(move || {
        let _ = tx.send(TrayCommand::CloseSettings);
        slint::CloseRequestResponse::HideWindow
    });
}
