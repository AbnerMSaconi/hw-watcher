// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Thermal, power, and memory alert bands matching the original panel.

use crate::model::Sample;

/// Visual severity band for a telemetry value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Band {
    /// Reading is absent.
    #[default]
    Muted,
    /// Reading is within normal operating limits.
    Normal,
    /// Reading crossed the warning threshold.
    Alert,
    /// Reading crossed the critical threshold.
    Critical,
}

/// Metrics that have defined warning/critical thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdMetric {
    /// CPU package temperature (°C).
    CpuTempC,
    /// CPU package power (W).
    CpuPowerW,
    /// GPU core temperature (°C).
    GpuTempC,
    /// GPU board power (W).
    GpuPowerW,
    /// VRAM junction temperature (°C).
    VramTempC,
    /// System RAM usage (%).
    RamPct,
    /// GPU VRAM usage (%).
    VramPct,
}

impl ThresholdMetric {
    /// Returns `(alert, critical)` inclusive thresholds.
    pub const fn limits(self) -> (f64, f64) {
        match self {
            Self::CpuTempC => (80.0, 90.0),
            Self::CpuPowerW => (100.0, 130.0),
            Self::GpuTempC => (75.0, 85.0),
            Self::GpuPowerW => (350.0, 380.0),
            Self::VramTempC => (85.0, 95.0),
            Self::RamPct | Self::VramPct => (80.0, 92.0),
        }
    }
}

/// Classifies an optional sensor reading against `metric`'s alert/critical thresholds.
pub fn classify(metric: ThresholdMetric, value: Option<f64>) -> Band {
    let Some(v) = value.filter(|v| v.is_finite()) else {
        return Band::Muted;
    };
    let (alert, critical) = metric.limits();
    if v >= critical {
        Band::Critical
    } else if v >= alert {
        Band::Alert
    } else {
        Band::Normal
    }
}

/// Convenience helper returning the severity band for a given [`ThresholdMetric`] on a [`Sample`].
pub fn classify_sample_metric(metric: ThresholdMetric, sample: &Sample) -> Band {
    let value = match metric {
        ThresholdMetric::CpuTempC => sample.cpu.tctl_c,
        ThresholdMetric::CpuPowerW => sample.cpu.package_w,
        ThresholdMetric::GpuTempC => sample.gpu.as_ref().and_then(|g| g.temp_c),
        ThresholdMetric::GpuPowerW => sample.gpu.as_ref().and_then(|g| g.power_w),
        ThresholdMetric::VramTempC => sample.gpu.as_ref().and_then(|g| g.vram_temp_c),
        ThresholdMetric::RamPct => sample.memory.used_pct(),
        ThresholdMetric::VramPct => sample
            .gpu
            .as_ref()
            .and_then(crate::model::GpuSample::vram_pct),
    };
    classify(metric, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_thresholds_consistently() {
        let cases = [
            (ThresholdMetric::GpuTempC, None, Band::Muted),
            (ThresholdMetric::GpuTempC, Some(40.0), Band::Normal),
            (ThresholdMetric::GpuTempC, Some(75.0), Band::Alert),
            (ThresholdMetric::GpuTempC, Some(85.0), Band::Critical),
            (ThresholdMetric::CpuPowerW, Some(99.9), Band::Normal),
            (ThresholdMetric::CpuPowerW, Some(100.0), Band::Alert),
            (ThresholdMetric::CpuPowerW, Some(130.0), Band::Critical),
            (ThresholdMetric::RamPct, Some(85.0), Band::Alert),
            (ThresholdMetric::VramPct, Some(92.0), Band::Critical),
        ];
        for (metric, val, expected) in cases {
            assert_eq!(classify(metric, val), expected);
        }
    }
}
