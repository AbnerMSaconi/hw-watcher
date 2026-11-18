// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Modular tray icon text and tooltip formatting, hiding unavailable sensors cleanly.

mod slots;
mod tooltip;

pub use slots::{TraySlotRendering, build_tray_slots};
pub use tooltip::{format_tooltip_lines, format_tooltip_lines_for};

use serde::{Deserialize, Serialize};

use crate::model::Sample;
use crate::thresholds::{Band, ThresholdMetric, classify};

/// Selectable metric rendered inside the compact system tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayMetric {
    /// Combined CPU + GPU watts (`CPU W` + `GPU W`).
    #[default]
    TotalPowerW,
    /// CPU package power in watts.
    CpuPowerW,
    /// GPU board power in watts.
    GpuPowerW,
    /// GPU graphics clock (`MHz` or `GHz` compactly).
    GpuClockMhz,
    /// System RAM utilization percentage.
    RamPct,
    /// GPU VRAM utilization percentage.
    VramPct,
    /// GPU core temperature in °C.
    GpuTempC,
    /// CPU Tctl temperature in °C.
    CpuTempC,
}

impl TrayMetric {
    /// All selectable tray icon metrics in menu display order.
    pub const ALL: [Self; 8] = [
        Self::TotalPowerW,
        Self::CpuPowerW,
        Self::GpuPowerW,
        Self::GpuClockMhz,
        Self::RamPct,
        Self::VramPct,
        Self::GpuTempC,
        Self::CpuTempC,
    ];

    /// Stable `snake_case` identifier used by configuration and UI state.
    pub const fn key(self) -> &'static str {
        match self {
            Self::TotalPowerW => "total_power",
            Self::CpuPowerW => "cpu_power",
            Self::GpuPowerW => "gpu_power",
            Self::GpuClockMhz => "gpu_clock",
            Self::RamPct => "ram_pct",
            Self::VramPct => "vram_pct",
            Self::GpuTempC => "gpu_temp",
            Self::CpuTempC => "cpu_temp",
        }
    }

    /// Parses a stable `key()` identifier back to [`TrayMetric`].
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == key)
    }

    /// English translation key shown in the tray menu and settings panel.
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::TotalPowerW => "Total power (W)",
            Self::CpuPowerW => "CPU power (W)",
            Self::GpuPowerW => "GPU power (W)",
            Self::GpuClockMhz => "GPU clock (MHz)",
            Self::RamPct => "RAM usage (%)",
            Self::VramPct => "VRAM usage (%)",
            Self::GpuTempC => "GPU temp (°C)",
            Self::CpuTempC => "CPU temp (°C)",
        }
    }

    /// Returns `true` when `sample` actually contains the data needed for this metric.
    pub fn is_available(self, sample: &Sample) -> bool {
        raw_value(self, sample).is_some()
    }

    /// Picks the configured metric if available, otherwise falls back to the first available one.
    #[must_use]
    pub fn resolve_for(self, sample: &Sample) -> Self {
        if self.is_available(sample) {
            return self;
        }
        Self::ALL
            .into_iter()
            .find(|m| m.is_available(sample))
            .unwrap_or(self)
    }
}

/// Modular tooltip visibility switches configured by the user.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TooltipSections {
    /// Show CPU/GPU/Total power lines.
    pub power: bool,
    /// Show CPU/GPU/VRAM/Hotspot temperature lines.
    pub temps: bool,
    /// Show RAM and VRAM usage lines.
    pub memory: bool,
    /// Show GPU clock, utilization, and power limit.
    pub clocks: bool,
    /// Show motherboard +12V voltage.
    pub board: bool,
}

impl Default for TooltipSections {
    fn default() -> Self {
        Self {
            power: true,
            temps: true,
            memory: true,
            clocks: true,
            board: true,
        }
    }
}

/// Output used to render the 32×32 SNI tray icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayRendering {
    /// Up to 4 compact characters (e.g. `"165"`, `"1.8G"`, `"42%"`, `"—"`).
    pub text: String,
    /// Severity band determining text color.
    pub band: Band,
}

/// Formats the compact icon string and color band for the selected [`TrayMetric`].
pub fn format_tray_rendering(preferred: TrayMetric, sample: &Sample) -> TrayRendering {
    let metric = preferred.resolve_for(sample);
    let Some(val) = raw_value(metric, sample) else {
        return TrayRendering {
            text: "—".into(),
            band: Band::Muted,
        };
    };
    format_rendering_for_value(metric, val, sample)
}

pub(crate) fn format_rendering_for_value(
    metric: TrayMetric,
    val: f64,
    sample: &Sample,
) -> TrayRendering {
    let text = match metric {
        TrayMetric::GpuClockMhz => format_compact_mhz(val),
        TrayMetric::RamPct | TrayMetric::VramPct => format!("{val:.0}%"),
        TrayMetric::TotalPowerW
        | TrayMetric::CpuPowerW
        | TrayMetric::GpuPowerW
        | TrayMetric::GpuTempC
        | TrayMetric::CpuTempC => format!("{val:.0}"),
    };
    TrayRendering {
        text,
        band: metric_band(metric, val, sample),
    }
}

pub(crate) fn raw_value(metric: TrayMetric, sample: &Sample) -> Option<f64> {
    match metric {
        TrayMetric::TotalPowerW => sample.total_power_w(),
        TrayMetric::CpuPowerW => {
            let mut vals = sample.effective_cpus().iter().filter_map(|c| c.package_w);
            let first = vals.next()?;
            Some(vals.fold(first, |a, b| a + b))
        }
        TrayMetric::GpuPowerW => {
            let mut vals = sample.effective_gpus().iter().filter_map(|g| g.power_w);
            let first = vals.next()?;
            Some(vals.fold(first, |a, b| a + b))
        }
        TrayMetric::GpuClockMhz => sample.gpu.as_ref()?.clock_mhz,
        TrayMetric::RamPct => sample.memory.used_pct(),
        TrayMetric::VramPct => sample.gpu.as_ref()?.vram_pct(),
        TrayMetric::GpuTempC => sample
            .effective_gpus()
            .iter()
            .filter_map(|g| g.temp_c)
            .reduce(f64::max),
        TrayMetric::CpuTempC => sample
            .effective_cpus()
            .iter()
            .filter_map(|c| c.tctl_c)
            .reduce(f64::max),
    }
}

pub(crate) fn metric_band(metric: TrayMetric, value: f64, sample: &Sample) -> Band {
    match metric {
        TrayMetric::TotalPowerW => {
            let gpu_w = sample.gpu.as_ref().and_then(|g| g.power_w);
            classify(ThresholdMetric::GpuPowerW, gpu_w)
        }
        TrayMetric::CpuPowerW => classify(ThresholdMetric::CpuPowerW, Some(value)),
        TrayMetric::GpuPowerW => classify(ThresholdMetric::GpuPowerW, Some(value)),
        TrayMetric::RamPct => classify(ThresholdMetric::RamPct, Some(value)),
        TrayMetric::VramPct => classify(ThresholdMetric::VramPct, Some(value)),
        TrayMetric::GpuTempC => classify(ThresholdMetric::GpuTempC, Some(value)),
        TrayMetric::CpuTempC => classify(ThresholdMetric::CpuTempC, Some(value)),
        TrayMetric::GpuClockMhz => Band::Normal,
    }
}

pub(crate) fn format_compact_mhz(mhz: f64) -> String {
    if mhz >= 1000.0 {
        format!("{:.1}G", mhz / 1000.0)
    } else {
        format!("{mhz:.0}")
    }
}

#[cfg(test)]
mod tests;
