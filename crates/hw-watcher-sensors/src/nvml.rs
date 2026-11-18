// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! NVIDIA GPU telemetry via NVML (`libnvidia-ml.so.1`), including VRAM used/total.

use hw_watcher_core::{GpuSample, GpuSource};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::enums::device::SampleValue;
use nvml_wrapper::structs::device::FieldId;
use nvml_wrapper::sys_exports::field_id::{NVML_FI_DEV_POWER_AVERAGE, NVML_FI_DEV_POWER_INSTANT};
use tracing::warn;

const BYTES_PER_MIB: f64 = 1024.0 * 1024.0;
const MW_PER_W: f64 = 1000.0;
const SLOW_POLL_TICKS: u64 = 10;

/// Metadata about the active NVML GPU used by BAR0 and NVAPI matching.
#[derive(Debug, Clone)]
pub struct GpuIdentity {
    /// Normalized sysfs PCI address (e.g. `"0000:2b:00.0"`).
    pub pci: String,
    /// PCI device/vendor ID (`(device << 16) | vendor`).
    pub pci_device_id: u32,
}

struct NvmlDeviceSlot {
    index: u32,
    cached_limit_w: Option<f64>,
    identity: GpuIdentity,
}

/// Reads GPU metrics for one or more NVIDIA GPUs from NVML without spawning `nvidia-smi`.
pub struct NvmlGpuSource {
    nvml: Nvml,
    tick: u64,
    slots: Vec<NvmlDeviceSlot>,
    identities: Vec<GpuIdentity>,
}

impl NvmlGpuSource {
    /// Opens NVML and discovers all NVIDIA GPUs (returns `None` when none exist).
    pub fn open() -> Option<Self> {
        let nvml = match Nvml::init() {
            Ok(lib) => lib,
            Err(e) => {
                warn!("NVML unavailable: {e}");
                return None;
            }
        };
        let count = nvml.device_count().ok().filter(|&c| c > 0)?;
        let mut slots = Vec::with_capacity(count as usize);
        let mut identities = Vec::with_capacity(count as usize);
        for index in 0..count {
            let Ok(dev) = nvml.device_by_index(index) else {
                continue;
            };
            let Ok(pci_info) = dev.pci_info() else {
                continue;
            };
            let identity = GpuIdentity {
                pci: normalize_pci(&pci_info.bus_id),
                pci_device_id: pci_info.pci_device_id,
            };
            identities.push(identity.clone());
            slots.push(NvmlDeviceSlot {
                index,
                cached_limit_w: None,
                identity,
            });
        }
        if slots.is_empty() {
            return None;
        }
        Some(Self {
            nvml,
            tick: 0,
            slots,
            identities,
        })
    }

    /// Returns the primary GPU's PCI identity.
    pub fn identity(&self) -> &GpuIdentity {
        &self.slots[0].identity
    }

    /// Returns PCI identities for all discovered GPUs.
    pub fn identities(&self) -> &[GpuIdentity] {
        &self.identities
    }

    /// Reads current telemetry for all discovered GPUs.
    pub fn read_all(&mut self) -> Vec<GpuSample> {
        let refresh_limit = self.tick.is_multiple_of(SLOW_POLL_TICKS);
        self.tick = self.tick.wrapping_add(1);
        let mut out = Vec::with_capacity(self.slots.len());
        for slot in &mut self.slots {
            let Ok(dev) = self.nvml.device_by_index(slot.index) else {
                continue;
            };
            if refresh_limit {
                slot.cached_limit_w = dev
                    .power_management_limit()
                    .ok()
                    .map(|mw| f64::from(mw) / MW_PER_W);
            }
            let (power_w, avg_power_w) = read_power(&dev);
            let mem = dev.memory_info().ok();
            out.push(GpuSample {
                temp_c: dev.temperature(TemperatureSensor::Gpu).ok().map(f64::from),
                power_w,
                avg_power_w,
                power_limit_w: slot.cached_limit_w,
                usage_pct: dev.utilization_rates().ok().map(|u| f64::from(u.gpu)),
                clock_mhz: dev.clock_info(Clock::Graphics).ok().map(f64::from),
                clock_mem_mhz: dev.clock_info(Clock::Memory).ok().map(f64::from),
                vram_used_mib: mem.as_ref().map(|m| (m.used as f64) / BYTES_PER_MIB),
                vram_total_mib: mem.as_ref().map(|m| (m.total as f64) / BYTES_PER_MIB),
                voltage_v: None,
                vram_temp_c: None,
                hotspot_temp_c: None,
            });
        }
        out
    }
}

impl GpuSource for NvmlGpuSource {
    fn read(&mut self) -> Option<GpuSample> {
        self.read_all().into_iter().next()
    }
}

fn read_power(dev: &nvml_wrapper::Device<'_>) -> (Option<f64>, Option<f64>) {
    if let Ok(entries) = dev.field_values_for(&[
        FieldId(NVML_FI_DEV_POWER_INSTANT),
        FieldId(NVML_FI_DEV_POWER_AVERAGE),
    ]) {
        let inst = entries
            .first()
            .and_then(|r| r.as_ref().ok())
            .and_then(|s| s.value.as_ref().ok())
            .and_then(mw_sample_to_w);
        let avg = entries
            .get(1)
            .and_then(|r| r.as_ref().ok())
            .and_then(|s| s.value.as_ref().ok())
            .and_then(mw_sample_to_w);
        if inst.is_some() || avg.is_some() {
            return (inst, avg);
        }
    }
    let fallback = dev.power_usage().ok().map(|mw| f64::from(mw) / MW_PER_W);
    (fallback, None)
}

fn mw_sample_to_w(sample: &SampleValue) -> Option<f64> {
    match sample {
        SampleValue::U32(mw) => Some(f64::from(*mw) / MW_PER_W),
        SampleValue::U64(mw) => Some((*mw as f64) / MW_PER_W),
        _ => None,
    }
}

/// Normalizes NVML `bus_id` (`00000000:2B:00.0`) to sysfs form (`0000:2b:00.0`).
pub fn normalize_pci(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    match lower.split_once(':') {
        Some((dom, rest)) if dom.len() > 4 => format!("{}:{rest}", &dom[dom.len() - 4..]),
        _ => lower,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_nvml_pci_bus_id() {
        assert_eq!(normalize_pci("00000000:2B:00.0"), "0000:2b:00.0");
        assert_eq!(normalize_pci("0000:01:00.0"), "0000:01:00.0");
    }
}
