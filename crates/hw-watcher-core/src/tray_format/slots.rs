// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Multi-slot taskbar rendering for up to N user-selected tray metrics.

use crate::model::Sample;
use crate::thresholds::{Band, ThresholdMetric, classify};

use super::{TrayMetric, TrayRendering, format_rendering_for_value, format_tray_rendering};

/// One rendered badge slot in the system tray / taskbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraySlotRendering {
    /// Metric shown in this slot.
    pub metric: TrayMetric,
    /// Optional device index when `separate_cpus` / `separate_gpus` expands a metric.
    pub device_index: Option<usize>,
    /// Compact badge text + severity band.
    pub rendering: TrayRendering,
}

/// Builds up to `max_items` tray icon badge renderings for `selected` metrics.
///
/// Returns an empty vector when `max_items == 0`.
pub fn build_tray_slots(
    selected: &[TrayMetric],
    max_items: usize,
    split_cpus: bool,
    separate_gpus: bool,
    sample: &Sample,
) -> Vec<TraySlotRendering> {
    if max_items == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(max_items);
    for &metric in selected {
        if out.len() >= max_items {
            break;
        }
        push_metric_slots(
            metric,
            max_items,
            split_cpus,
            separate_gpus,
            sample,
            &mut out,
        );
    }
    if out.is_empty() {
        let fallback = selected.first().copied().unwrap_or(TrayMetric::TotalPowerW);
        let resolved = fallback.resolve_for(sample);
        out.push(TraySlotRendering {
            metric: resolved,
            device_index: None,
            rendering: format_tray_rendering(fallback, sample),
        });
    }
    out
}

fn push_metric_slots(
    metric: TrayMetric,
    max_items: usize,
    split_cpus: bool,
    separate_gpus: bool,
    sample: &Sample,
    out: &mut Vec<TraySlotRendering>,
) {
    let cpus = sample.effective_cpus();
    let gpus = sample.effective_gpus();
    if split_cpus
        && cpus.len() > 1
        && matches!(metric, TrayMetric::CpuPowerW | TrayMetric::CpuTempC)
    {
        for (idx, cpu) in cpus.iter().enumerate() {
            if out.len() >= max_items {
                break;
            }
            let val = match metric {
                TrayMetric::CpuPowerW => cpu.package_w,
                TrayMetric::CpuTempC => cpu.tctl_c,
                _ => None,
            };
            if let Some(v) = val {
                out.push(TraySlotRendering {
                    metric,
                    device_index: Some(idx),
                    rendering: format_rendering_for_value(metric, v, sample),
                });
            }
        }
        return;
    }

    if separate_gpus
        && gpus.len() > 1
        && matches!(
            metric,
            TrayMetric::GpuPowerW
                | TrayMetric::GpuClockMhz
                | TrayMetric::VramPct
                | TrayMetric::GpuTempC
        )
    {
        for (idx, gpu) in gpus.iter().enumerate() {
            if out.len() >= max_items {
                break;
            }
            let (val, band_override) = match metric {
                TrayMetric::GpuPowerW => (
                    gpu.power_w,
                    gpu.power_w
                        .map(|w| classify(ThresholdMetric::GpuPowerW, Some(w))),
                ),
                TrayMetric::GpuClockMhz => (gpu.clock_mhz, Some(Band::Normal)),
                TrayMetric::VramPct => (
                    gpu.vram_pct(),
                    gpu.vram_pct()
                        .map(|p| classify(ThresholdMetric::VramPct, Some(p))),
                ),
                TrayMetric::GpuTempC => (
                    gpu.temp_c,
                    gpu.temp_c
                        .map(|t| classify(ThresholdMetric::GpuTempC, Some(t))),
                ),
                _ => (None, None),
            };
            if let Some(v) = val {
                let mut rendering = format_rendering_for_value(metric, v, sample);
                if let Some(b) = band_override {
                    rendering.band = b;
                }
                out.push(TraySlotRendering {
                    metric,
                    device_index: Some(idx),
                    rendering,
                });
            }
        }
        return;
    }

    if metric.is_available(sample) {
        out.push(TraySlotRendering {
            metric,
            device_index: None,
            rendering: format_tray_rendering(metric, sample),
        });
    }
}
