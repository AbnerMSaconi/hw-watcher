// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Persistent user preferences (`$XDG_CONFIG_HOME/hw-watcher/config.toml`).

use std::fs;
use std::path::{Path, PathBuf};

use hw_watcher_core::{TooltipSections, TrayMetric};
use serde::{Deserialize, Serialize};

use crate::theme::ThemeConfig;

const DEFAULT_X: i32 = 60;
const DEFAULT_Y: i32 = 60;
const DEFAULT_INTERVAL_MS: u64 = 1000;
const MIN_INTERVAL_MS: u64 = 250;
const MAX_TRAY_ITEMS: u8 = 8;

/// Visibility switches for metrics shown inside the floating widget.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetItemsConfig {
    /// Show source origin badge (`daemon` / `local` / `sim`) in widget header.
    pub show_source_badge: bool,
    /// Show CPU row(s) in floating widget.
    pub cpu: bool,
    /// Show GPU row(s) in floating widget.
    pub gpu: bool,
    /// Show combined Total power / peak temperature row.
    pub total: bool,
    /// Show system RAM row.
    pub ram: bool,
    /// Show GPU VRAM row(s).
    pub vram: bool,
    /// Show GPU clocks, utilization, and power limit in footer.
    pub gpu_clocks: bool,
    /// Show elevated GPU VRAM/hotspot temps and core voltage in footer.
    pub gpu_elevated: bool,
    /// Show motherboard +12V rail in footer.
    pub board_v12: bool,
    /// Show extended motherboard voltage rails (+5V, +3.3V, Vcore, `SoC`, DRAM) in footer.
    pub board_rails: bool,
    /// Show motherboard temperatures (VRM MOS, Socket, System, PCH, `PCIe`) in footer.
    pub board_temps: bool,
    /// Show motherboard fan RPMs (CPU, Sys1, Sys3) in footer.
    pub board_fans: bool,
}

impl Default for WidgetItemsConfig {
    fn default() -> Self {
        Self {
            show_source_badge: true,
            cpu: true,
            gpu: true,
            total: true,
            ram: true,
            vram: true,
            gpu_clocks: true,
            gpu_elevated: true,
            board_v12: true,
            board_rails: false,
            board_temps: false,
            board_fans: false,
        }
    }
}

/// Window geometry, multi-device display, taskbar routing, widget visibility, and theme.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    /// Saved X position in physical pixels.
    pub x: i32,
    /// Saved Y position in physical pixels.
    pub y: i32,
    /// Keep floating window above other windows.
    pub always_on_top: bool,
    /// Hide the floating panel from the desktop taskbar and pager.
    pub skip_taskbar: bool,
    /// Start window visible (or hidden in tray if `false`).
    pub visible: bool,
    /// Local polling interval in milliseconds.
    pub interval_ms: u64,
    /// List multiple CPUs separately when more than one CPU is detected.
    pub separate_cpus: bool,
    /// List multiple GPUs separately when more than one GPU is detected.
    pub separate_gpus: bool,
    /// Primary metric displayed inside the first tray badge.
    pub tray_metric: TrayMetric,
    /// How many metric badges to display on the taskbar (`0..=8`).
    pub tray_count: u8,
    /// Ordered list of metrics enabled for the taskbar.
    pub tray_metrics: Vec<TrayMetric>,
    /// Tooltip sections enabled by the user.
    pub tooltip: TooltipSections,
    /// Floating widget item visibility switches.
    pub widget_items: WidgetItemsConfig,
    /// Customizable colors, transparency, and blur settings.
    pub theme: ThemeConfig,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            x: DEFAULT_X,
            y: DEFAULT_Y,
            always_on_top: true,
            skip_taskbar: true,
            visible: true,
            interval_ms: DEFAULT_INTERVAL_MS,
            separate_cpus: true,
            separate_gpus: true,
            tray_metric: TrayMetric::default(),
            tray_count: 1,
            tray_metrics: vec![
                TrayMetric::TotalPowerW,
                TrayMetric::GpuTempC,
                TrayMetric::CpuTempC,
                TrayMetric::RamPct,
                TrayMetric::VramPct,
            ],
            tooltip: TooltipSections::default(),
            widget_items: WidgetItemsConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

impl PanelConfig {
    /// Returns the default configuration file path.
    pub fn default_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map_or_else(
                || PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"),
                PathBuf::from,
            );
        base.join("hw-watcher/config.toml")
    }

    /// Loads config from `default_path()`, falling back to defaults if absent or invalid.
    pub fn load() -> Self {
        Self::load_from(&Self::default_path())
    }

    /// Loads config from a specific TOML path.
    pub fn load_from(path: &Path) -> Self {
        let mut cfg: Self = fs::read_to_string(path)
            .ok()
            .and_then(|raw| toml::from_str(&raw).ok())
            .unwrap_or_default();
        cfg.sanitize();
        cfg
    }

    /// Normalizes intervals, clamps counts, and ensures tray metrics stay consistent.
    pub fn sanitize(&mut self) {
        self.interval_ms = self.interval_ms.max(MIN_INTERVAL_MS);
        self.tray_count = self.tray_count.min(MAX_TRAY_ITEMS);
        if self.tray_metrics.is_empty() {
            self.tray_metrics.push(self.tray_metric);
        } else if !self.tray_metrics.contains(&self.tray_metric) {
            self.tray_metric = self.tray_metrics[0];
        }
        self.theme.sanitize();
    }

    /// Returns the ordered tray metrics starting with `self.tray_metric` when present.
    pub fn ordered_tray_metrics(&self) -> Vec<TrayMetric> {
        let mut out = Vec::with_capacity(self.tray_metrics.len() + 1);
        if self.tray_metrics.contains(&self.tray_metric) {
            out.push(self.tray_metric);
        }
        for &m in &self.tray_metrics {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        if out.is_empty() {
            out.push(self.tray_metric);
        }
        out
    }

    /// Atomically saves config to `default_path()`.
    pub fn save(&self) {
        self.save_to(&Self::default_path());
    }

    /// Atomically saves config to `path`.
    pub fn save_to(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(encoded) = toml::to_string_pretty(self) {
            let tmp = path.with_extension("tmp");
            if fs::write(&tmp, encoded).is_ok() {
                let _ = fs::rename(tmp, path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_panel_config_and_enforces_min_interval() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut cfg = PanelConfig {
            x: 120,
            y: 240,
            always_on_top: false,
            skip_taskbar: false,
            visible: true,
            interval_ms: 50,
            separate_cpus: true,
            separate_gpus: true,
            tray_metric: TrayMetric::RamPct,
            tray_count: 3,
            tray_metrics: vec![TrayMetric::RamPct, TrayMetric::GpuTempC],
            tooltip: TooltipSections {
                power: true,
                temps: false,
                memory: true,
                clocks: false,
                board: true,
            },
            ..Default::default()
        };
        cfg.theme.blur_enabled = true;
        cfg.theme.bg_opacity_pct = 65;
        cfg.theme
            .item_label_colors
            .insert("cpu".into(), "#56d4dd".into());
        cfg.save_to(&path);

        let loaded = PanelConfig::load_from(&path);
        assert_eq!(loaded.x, 120);
        assert_eq!(loaded.y, 240);
        assert!(!loaded.always_on_top);
        assert!(!loaded.skip_taskbar);
        assert!(loaded.separate_cpus);
        assert!(loaded.separate_gpus);
        assert_eq!(loaded.interval_ms, MIN_INTERVAL_MS);
        assert_eq!(loaded.tray_metric, TrayMetric::RamPct);
        assert_eq!(loaded.tray_count, 3);
        assert!(!loaded.tooltip.temps);
        assert!(loaded.theme.blur_enabled);
        assert_eq!(loaded.theme.bg_opacity_pct, 65);
        assert_eq!(loaded.theme.label_color_hex("cpu"), "#56d4dd");
    }
}
