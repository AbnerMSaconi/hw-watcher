// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Slint property synchronization for `SettingsWindow`.

use hw_watcher_core::{Sample, Translator};
use slint::{Color, ModelRc, VecModel};

use crate::app::{ItemColorRow, SettingsWindow, TrayMetricRow};
use crate::config::PanelConfig;
use crate::i18n::PoTranslator;
use crate::settings_state::{get_target_hex, listed_item_keys, ordered_tray_metric_rows};
use crate::theme::{color_with_opacity, parse_hex_rgb};

/// Updates only the hardware detection summary and per-item color list.
pub fn sync_hardware_summary(w: &SettingsWindow, cfg: &PanelConfig, sample: &Sample) {
    let tr = PoTranslator::active();
    let cpus = sample.effective_cpus().len();
    let gpus = sample.effective_gpus().len();
    w.set_hardware_summary(
        tr.format(
            "{cpus} CPU · {gpus} GPU",
            &[("cpus", &cpus.to_string()), ("gpus", &gpus.to_string())],
        )
        .into(),
    );
    sync_item_color_rows(w, cfg, sample, &tr);
}

/// Syncs all `PanelConfig` fields, color swatches, and lists into `SettingsWindow`.
pub fn sync_window(w: &SettingsWindow, cfg: &PanelConfig, sample: &Sample) {
    let tr = PoTranslator::active();
    w.set_app_version(env!("CARGO_PKG_VERSION").into());
    w.set_legal_notice(crate::about::short_notice().into());
    w.set_cfg_visible(cfg.visible);
    w.set_cfg_always_on_top(cfg.always_on_top);
    w.set_cfg_skip_taskbar(cfg.skip_taskbar);
    w.set_cfg_interval_ms(cfg.interval_ms as i32);
    w.set_cfg_separate_cpus(cfg.separate_cpus);
    w.set_cfg_separate_gpus(cfg.separate_gpus);
    w.set_cfg_tray_count(i32::from(cfg.tray_count));

    w.set_tip_power(cfg.tooltip.power);
    w.set_tip_temps(cfg.tooltip.temps);
    w.set_tip_memory(cfg.tooltip.memory);
    w.set_tip_clocks(cfg.tooltip.clocks);
    w.set_tip_board(cfg.tooltip.board);

    let wi = &cfg.widget_items;
    w.set_w_badge(wi.show_source_badge);
    w.set_w_cpu(wi.cpu);
    w.set_w_gpu(wi.gpu);
    w.set_w_total(wi.total);
    w.set_w_ram(wi.ram);
    w.set_w_vram(wi.vram);
    w.set_w_gpu_clocks(wi.gpu_clocks);
    w.set_w_gpu_elevated(wi.gpu_elevated);
    w.set_w_board_v12(wi.board_v12);
    w.set_w_board_rails(wi.board_rails);
    w.set_w_board_temps(wi.board_temps);
    w.set_w_board_fans(wi.board_fans);

    let th = &cfg.theme;
    w.set_th_bg_color(color_with_opacity(&th.bg_color, 100, [0x14, 0x14, 0x18]));
    w.set_th_bg_hex(th.bg_color.clone().into());
    w.set_th_bg_opacity(i32::from(th.bg_opacity_pct));
    w.set_th_border_color(color_with_opacity(
        &th.border_color,
        100,
        [0xff, 0xff, 0xff],
    ));
    w.set_th_border_hex(th.border_color.clone().into());
    w.set_th_border_opacity(i32::from(th.border_opacity_pct));
    w.set_th_blur_enabled(th.blur_enabled);
    w.set_th_tray_bg_color(color_with_opacity(
        &th.tray_bg_color,
        100,
        [0x14, 0x14, 0x18],
    ));
    w.set_th_tray_bg_hex(th.tray_bg_color.clone().into());
    w.set_th_tray_bg_opacity(i32::from(th.tray_bg_opacity_pct));
    w.set_th_text_color(th.text_slint());
    w.set_th_text_hex(th.text_color.clone().into());
    w.set_th_muted_color(th.muted_slint());
    w.set_th_muted_hex(th.muted_color.clone().into());
    w.set_th_alert_color(color_with_opacity(&th.alert_color, 100, [0xff, 0xb3, 0x47]));
    w.set_th_alert_hex(th.alert_color.clone().into());
    w.set_th_critical_color(color_with_opacity(
        &th.critical_color,
        100,
        [0xff, 0x5c, 0x5c],
    ));
    w.set_th_critical_hex(th.critical_color.clone().into());

    let tm_rows: Vec<TrayMetricRow> = ordered_tray_metric_rows(cfg)
        .into_iter()
        .map(|(m, enabled)| TrayMetricRow {
            key: m.key().into(),
            label: tr.translate(m.label_key()).as_ref().into(),
            enabled,
        })
        .collect();
    w.set_tray_metric_rows(ModelRc::new(VecModel::from(tm_rows)));

    sync_hardware_summary(w, cfg, sample);
    let target = w.get_active_color_target().to_string();
    sync_active_color(w, cfg, &target);
}

fn sync_item_color_rows(
    w: &SettingsWindow,
    cfg: &PanelConfig,
    sample: &Sample,
    tr: &dyn Translator,
) {
    let rows: Vec<ItemColorRow> = listed_item_keys(cfg, sample, tr)
        .into_iter()
        .map(|(key, label)| {
            let hex = cfg.theme.label_color_hex(&key).to_owned();
            let is_custom = cfg.theme.item_label_colors.contains_key(&key);
            ItemColorRow {
                color: cfg.theme.label_color(&key),
                key: key.into(),
                label: label.into(),
                hex: hex.into(),
                is_custom,
            }
        })
        .collect();
    w.set_item_color_rows(ModelRc::new(VecModel::from(rows)));
}

/// Syncs the inline RGB editor preview for `target`.
pub fn sync_active_color(w: &SettingsWindow, cfg: &PanelConfig, target: &str) {
    let hex = get_target_hex(cfg, target);
    let rgb = parse_hex_rgb(&hex).unwrap_or([0x14, 0x14, 0x18]);
    w.set_active_color_hex(hex.into());
    w.set_active_color_preview(Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
    w.set_active_r(i32::from(rgb[0]));
    w.set_active_g(i32::from(rgb[1]));
    w.set_active_b(i32::from(rgb[2]));
}
