// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Per-device wire structs for multi-CPU and multi-GPU D-Bus transport.

use hw_watcher_core::{CpuSample, GpuSample};
use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

/// Encodes `Option<f64>` as `f64::NAN` for D-Bus wire transfer.
pub(crate) fn wire_opt(v: Option<f64>) -> f64 {
    v.unwrap_or(f64::NAN)
}

/// Decodes `f64::NAN` back to `None`.
pub(crate) fn opt_wire(v: f64) -> Option<f64> {
    v.is_finite().then_some(v)
}

/// D-Bus wire encoding of one [`CpuSample`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct WireCpuSample {
    /// CPU package power in W (`f64::NAN` when unavailable).
    pub package_w: f64,
    /// CPU Tctl temperature in °C (`f64::NAN` when unavailable).
    pub tctl_c: f64,
}

impl From<&CpuSample> for WireCpuSample {
    fn from(c: &CpuSample) -> Self {
        Self {
            package_w: wire_opt(c.package_w),
            tctl_c: wire_opt(c.tctl_c),
        }
    }
}

impl From<WireCpuSample> for CpuSample {
    fn from(w: WireCpuSample) -> Self {
        Self {
            package_w: opt_wire(w.package_w),
            tctl_c: opt_wire(w.tctl_c),
        }
    }
}

/// D-Bus wire encoding of one [`GpuSample`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Type)]
pub struct WireGpuSample {
    /// GPU core temperature in °C.
    pub temp_c: f64,
    /// GPU instantaneous power draw in W.
    pub power_w: f64,
    /// GPU averaged power draw in W.
    pub avg_power_w: f64,
    /// GPU power limit in W.
    pub power_limit_w: f64,
    /// GPU utilization in %.
    pub usage_pct: f64,
    /// GPU graphics clock in MHz.
    pub clock_mhz: f64,
    /// GPU memory clock in MHz.
    pub clock_mem_mhz: f64,
    /// GPU VRAM used in MiB.
    pub vram_used_mib: f64,
    /// GPU VRAM total in MiB.
    pub vram_total_mib: f64,
    /// GPU core voltage in V.
    pub voltage_v: f64,
    /// GPU VRAM junction temperature in °C.
    pub vram_temp_c: f64,
    /// GPU hotspot temperature in °C.
    pub hotspot_temp_c: f64,
}

impl From<&GpuSample> for WireGpuSample {
    fn from(g: &GpuSample) -> Self {
        Self {
            temp_c: wire_opt(g.temp_c),
            power_w: wire_opt(g.power_w),
            avg_power_w: wire_opt(g.avg_power_w),
            power_limit_w: wire_opt(g.power_limit_w),
            usage_pct: wire_opt(g.usage_pct),
            clock_mhz: wire_opt(g.clock_mhz),
            clock_mem_mhz: wire_opt(g.clock_mem_mhz),
            vram_used_mib: wire_opt(g.vram_used_mib),
            vram_total_mib: wire_opt(g.vram_total_mib),
            voltage_v: wire_opt(g.voltage_v),
            vram_temp_c: wire_opt(g.vram_temp_c),
            hotspot_temp_c: wire_opt(g.hotspot_temp_c),
        }
    }
}

impl From<WireGpuSample> for GpuSample {
    fn from(w: WireGpuSample) -> Self {
        Self {
            temp_c: opt_wire(w.temp_c),
            power_w: opt_wire(w.power_w),
            avg_power_w: opt_wire(w.avg_power_w),
            power_limit_w: opt_wire(w.power_limit_w),
            usage_pct: opt_wire(w.usage_pct),
            clock_mhz: opt_wire(w.clock_mhz),
            clock_mem_mhz: opt_wire(w.clock_mem_mhz),
            vram_used_mib: opt_wire(w.vram_used_mib),
            vram_total_mib: opt_wire(w.vram_total_mib),
            voltage_v: opt_wire(w.voltage_v),
            vram_temp_c: opt_wire(w.vram_temp_c),
            hotspot_temp_c: opt_wire(w.hotspot_temp_c),
        }
    }
}
