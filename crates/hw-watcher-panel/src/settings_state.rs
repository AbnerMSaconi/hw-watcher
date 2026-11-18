// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Pure configuration mutation and item-list builders for the Control Panel.

use hw_watcher_core::{Sample, Translator, TrayMetric};

use crate::config::PanelConfig;
use crate::theme::{format_hex_rgb, parse_hex_rgb};

/// Applies a named boolean toggle from `SettingsWindow` to `cfg`.
pub fn apply_bool_setting(cfg: &mut PanelConfig, key: &str, value: bool) {
    match key {
        "visible" => cfg.visible = value,
        "always_on_top" => cfg.always_on_top = value,
        "skip_taskbar" => cfg.skip_taskbar = value,
        "separate_cpus" => cfg.separate_cpus = value,
        "separate_gpus" => cfg.separate_gpus = value,
        "blur_enabled" => cfg.theme.blur_enabled = value,
        "tip_power" => cfg.tooltip.power = value,
        "tip_temps" => cfg.tooltip.temps = value,
        "tip_memory" => cfg.tooltip.memory = value,
        "tip_clocks" => cfg.tooltip.clocks = value,
        "tip_board" => cfg.tooltip.board = value,
        "w_badge" => cfg.widget_items.show_source_badge = value,
        "w_cpu" => cfg.widget_items.cpu = value,
        "w_gpu" => cfg.widget_items.gpu = value,
        "w_total" => cfg.widget_items.total = value,
        "w_ram" => cfg.widget_items.ram = value,
        "w_vram" => cfg.widget_items.vram = value,
        "w_gpu_clocks" => cfg.widget_items.gpu_clocks = value,
        "w_gpu_elevated" => cfg.widget_items.gpu_elevated = value,
        "w_board_v12" => cfg.widget_items.board_v12 = value,
        "w_board_rails" => cfg.widget_items.board_rails = value,
        "w_board_temps" => cfg.widget_items.board_temps = value,
        "w_board_fans" => cfg.widget_items.board_fans = value,
        _ => {}
    }
}

/// Applies a named integer setting (`interval_ms`, `tray_count`, opacities) to `cfg`.
pub fn apply_int_setting(cfg: &mut PanelConfig, key: &str, value: i32) {
    let clamped_pct = value.clamp(0, 100) as u8;
    match key {
        "interval_ms" => cfg.interval_ms = u64::try_from(value.max(250)).unwrap_or(1000),
        "tray_count" => cfg.tray_count = value.clamp(0, 8) as u8,
        "bg_opacity" => cfg.theme.bg_opacity_pct = clamped_pct,
        "border_opacity" => cfg.theme.border_opacity_pct = clamped_pct,
        "tray_bg_opacity" => cfg.theme.tray_bg_opacity_pct = clamped_pct,
        _ => {}
    }
    cfg.sanitize();
}

/// Toggles a `TrayMetric` in `cfg.tray_metrics`.
pub fn toggle_tray_metric(cfg: &mut PanelConfig, key: &str) {
    let Some(metric) = TrayMetric::from_key(key) else {
        return;
    };
    if let Some(pos) = cfg.tray_metrics.iter().position(|&m| m == metric) {
        if cfg.tray_metrics.len() > 1 {
            cfg.tray_metrics.remove(pos);
        }
    } else {
        cfg.tray_metrics.push(metric);
    }
    cfg.tray_metric = cfg.tray_metrics[0];
    cfg.sanitize();
}

/// Moves a `TrayMetric` up (`delta = -1`) or down (`delta = 1`) in taskbar display priority.
pub fn move_tray_metric(cfg: &mut PanelConfig, key: &str, delta: i32) {
    let Some(metric) = TrayMetric::from_key(key) else {
        return;
    };
    let Some(idx) = cfg.tray_metrics.iter().position(|&m| m == metric) else {
        return;
    };
    let target = if delta < 0 {
        idx.saturating_sub(1)
    } else {
        (idx + 1).min(cfg.tray_metrics.len() - 1)
    };
    cfg.tray_metrics.swap(idx, target);
    cfg.tray_metric = cfg.tray_metrics[0];
}

/// Returns ordered `(metric, enabled)` rows for the taskbar metric selector.
pub fn ordered_tray_metric_rows(cfg: &PanelConfig) -> Vec<(TrayMetric, bool)> {
    let mut rows = Vec::with_capacity(TrayMetric::ALL.len());
    for &m in &cfg.tray_metrics {
        rows.push((m, true));
    }
    for m in TrayMetric::ALL {
        if !cfg.tray_metrics.contains(&m) {
            rows.push((m, false));
        }
    }
    rows
}

/// Reads the `#rrggbb` hex string for a color target (`bg`, `border`, `text`, `item:cpu`, ...).
pub fn get_target_hex(cfg: &PanelConfig, target: &str) -> String {
    match target {
        "bg" => cfg.theme.bg_color.clone(),
        "border" => cfg.theme.border_color.clone(),
        "tray_bg" => cfg.theme.tray_bg_color.clone(),
        "text" => cfg.theme.text_color.clone(),
        "muted" => cfg.theme.muted_color.clone(),
        "alert" => cfg.theme.alert_color.clone(),
        "critical" => cfg.theme.critical_color.clone(),
        _ => target.strip_prefix("item:").map_or_else(
            || cfg.theme.text_color.clone(),
            |k| cfg.theme.label_color_hex(k).to_owned(),
        ),
    }
}

/// Writes `#rrggbb` to the selected color target in `cfg.theme`.
pub fn set_target_hex(cfg: &mut PanelConfig, target: &str, hex: &str) {
    let Some(rgb) = parse_hex_rgb(hex) else {
        return;
    };
    let norm = format_hex_rgb(rgb);
    match target {
        "bg" => cfg.theme.bg_color = norm,
        "border" => cfg.theme.border_color = norm,
        "tray_bg" => cfg.theme.tray_bg_color = norm,
        "text" => cfg.theme.text_color = norm,
        "muted" => cfg.theme.muted_color = norm,
        "alert" => cfg.theme.alert_color = norm,
        "critical" => cfg.theme.critical_color = norm,
        _ => {
            if let Some(key) = target.strip_prefix("item:") {
                cfg.theme.item_label_colors.insert(key.into(), norm);
            }
        }
    }
}

/// Lists all per-item label keys and display titles according to hardware count & separation.
pub fn listed_item_keys(
    cfg: &PanelConfig,
    sample: &Sample,
    tr: &dyn Translator,
) -> Vec<(String, String)> {
    let mut items = Vec::new();
    let cpu_count = sample.effective_cpus().len();
    if cfg.separate_cpus && cpu_count > 1 {
        for i in 0..cpu_count {
            items.push((format!("cpu:{i}"), format!("CPU {i}")));
        }
    } else {
        items.push(("cpu".into(), "CPU".into()));
    }

    let gpu_count = sample.effective_gpus().len();
    if cfg.separate_gpus && gpu_count > 1 {
        for i in 0..gpu_count {
            items.push((format!("gpu:{i}"), format!("GPU {i}")));
        }
    } else {
        items.push(("gpu".into(), "GPU".into()));
    }

    items.push(("total".into(), tr.translate("Total").into_owned()));
    items.push(("ram".into(), tr.translate("RAM").into_owned()));

    if cfg.separate_gpus && gpu_count > 1 {
        for i in 0..gpu_count {
            items.push((format!("vram:{i}"), format!("{} {i}", tr.translate("VRAM"))));
        }
    } else {
        items.push(("vram".into(), tr.translate("VRAM").into_owned()));
    }

    items.push(("board".into(), tr.translate("Board / Footer").into_owned()));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use hw_watcher_core::{CpuSample, GpuSample, IdentityTranslator};

    #[test]
    fn toggle_and_reorder_tray_metrics_and_item_colors() {
        let mut cfg = PanelConfig {
            tray_metrics: vec![TrayMetric::TotalPowerW, TrayMetric::GpuTempC],
            ..Default::default()
        };
        move_tray_metric(&mut cfg, "gpu_temp", -1);
        assert_eq!(
            cfg.tray_metrics,
            vec![TrayMetric::GpuTempC, TrayMetric::TotalPowerW]
        );

        set_target_hex(&mut cfg, "item:cpu:0", "#56D4DD");
        assert_eq!(get_target_hex(&cfg, "item:cpu:0"), "#56d4dd");

        let sample = Sample {
            cpus: vec![CpuSample::default(), CpuSample::default()],
            gpus: vec![GpuSample::default(), GpuSample::default()],
            ..Default::default()
        };
        cfg.separate_cpus = true;
        cfg.separate_gpus = true;
        let keys = listed_item_keys(&cfg, &sample, &IdentityTranslator);
        assert!(keys.iter().any(|(k, _)| k == "cpu:1"));
        assert!(keys.iter().any(|(k, _)| k == "gpu:1"));
        assert!(keys.iter().any(|(k, _)| k == "vram:1"));
    }
}
