// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! D-Bus contract between the optional `hw-watcher daemon` and desktop clients.
//!
//! Missing sensor values cross D-Bus as `f64::NAN` and convert back to `None` via [`From<WireSample>`].

#![forbid(unsafe_code)]

mod devices;

pub use devices::{WireCpuSample, WireGpuSample};
use devices::{opt_wire, wire_opt};
use hw_watcher_core::{
    BoardSample, Capabilities, CpuSample, GpuSample, MemorySample, Sample, SourceOrigin,
};
use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

/// Well-known D-Bus service name.
pub const BUS_NAME: &str = "io.github.abnermsaconi.HwWatcher";
/// Well-known D-Bus object path.
pub const OBJECT_PATH: &str = "/io/github/abnermsaconi/HwWatcher";
/// Well-known D-Bus interface name.
pub const INTERFACE: &str = "io.github.abnermsaconi.HwWatcher1";

/// Returns the system D-Bus address (`DBUS_SYSTEM_BUS_ADDRESS` or `/run/dbus/system_bus_socket`).
pub fn system_bus_address() -> String {
    match std::env::var("DBUS_SYSTEM_BUS_ADDRESS") {
        Ok(addr) if !addr.trim().is_empty() => addr,
        _ => "unix:path=/run/dbus/system_bus_socket".into(),
    }
}

/// D-Bus wire encoding of [`Capabilities`].
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
pub struct WireCapabilities {
    /// NVML GPU telemetry available.
    pub nvml: bool,
    /// RAPL CPU package power readable.
    pub rapl: bool,
    /// `k10temp` CPU temperature readable.
    pub hwmon_cpu: bool,
    /// `nct6687` motherboard telemetry readable.
    pub hwmon_board: bool,
    /// `/proc/meminfo` system RAM readable.
    pub meminfo: bool,
    /// BAR0 VRAM/hotspot temperatures readable.
    pub bar0: bool,
    /// NVAPI core voltage readable.
    pub nvapi_voltage: bool,
}

impl From<Capabilities> for WireCapabilities {
    fn from(c: Capabilities) -> Self {
        Self {
            nvml: c.nvml,
            rapl: c.rapl,
            hwmon_cpu: c.hwmon_cpu,
            hwmon_board: c.hwmon_board,
            meminfo: c.meminfo,
            bar0: c.bar0,
            nvapi_voltage: c.nvapi_voltage,
        }
    }
}

impl From<WireCapabilities> for Capabilities {
    fn from(c: WireCapabilities) -> Self {
        Self {
            nvml: c.nvml,
            rapl: c.rapl,
            hwmon_cpu: c.hwmon_cpu,
            hwmon_board: c.hwmon_board,
            meminfo: c.meminfo,
            bar0: c.bar0,
            nvapi_voltage: c.nvapi_voltage,
        }
    }
}

/// D-Bus wire encoding of [`Sample`] (`f64::NAN` represents `None`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct WireSample {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Active capabilities when sampled.
    pub caps: WireCapabilities,
    /// Whether GPU fields are populated.
    pub has_gpu: bool,
    /// GPU core temperature in °C.
    pub gpu_temp_c: f64,
    /// GPU instantaneous power draw in W.
    pub gpu_power_w: f64,
    /// GPU averaged power draw in W.
    pub gpu_avg_power_w: f64,
    /// GPU power limit in W.
    pub gpu_power_limit_w: f64,
    /// GPU utilization in %.
    pub gpu_usage_pct: f64,
    /// GPU graphics clock in MHz.
    pub gpu_clock_mhz: f64,
    /// GPU memory clock in MHz.
    pub gpu_clock_mem_mhz: f64,
    /// GPU VRAM used in MiB.
    pub vram_used_mib: f64,
    /// GPU VRAM total in MiB.
    pub vram_total_mib: f64,
    /// GPU core voltage in V.
    pub gpu_voltage_v: f64,
    /// GPU VRAM junction temperature in °C.
    pub vram_temp_c: f64,
    /// GPU hotspot temperature in °C.
    pub hotspot_temp_c: f64,
    /// CPU package power in W.
    pub cpu_package_w: f64,
    /// CPU Tctl temperature in °C.
    pub cpu_tctl_c: f64,
    /// System RAM used in MiB.
    pub ram_used_mib: f64,
    /// System RAM total in MiB.
    pub ram_total_mib: f64,
    /// Motherboard +12V rail in V.
    pub board_v12_v: f64,
    /// Motherboard +5V rail in V.
    pub board_v5_v: f64,
    /// Motherboard +3.3V rail in V.
    pub board_v3_3_v: f64,
    /// CPU Vcore in V.
    pub board_vcore_v: f64,
    /// CPU `SoC` voltage in V.
    pub board_soc_v: f64,
    /// DRAM voltage in V.
    pub board_dram_v: f64,
    /// VRM MOS temperature in °C.
    pub board_vrm_mos_c: f64,
    /// CPU socket temperature in °C.
    pub board_cpu_socket_c: f64,
    /// System temperature in °C.
    pub board_system_c: f64,
    /// PCH temperature in °C.
    pub board_pch_c: f64,
    /// `PCIe` x1 temperature in °C.
    pub board_pcie_x1_c: f64,
    /// CPU fan speed in RPM.
    pub board_cpu_fan_rpm: f64,
    /// System fan 1 speed in RPM.
    pub board_sys_fan1_rpm: f64,
    /// System fan 3 speed in RPM.
    pub board_sys_fan3_rpm: f64,
    /// Per-CPU snapshots.
    pub cpus: Vec<WireCpuSample>,
    /// Per-GPU snapshots.
    pub gpus: Vec<WireGpuSample>,
}

impl From<&Sample> for WireSample {
    fn from(s: &Sample) -> Self {
        let g = s.gpu.as_ref();
        Self {
            timestamp_ms: s.timestamp_ms,
            caps: s.caps.into(),
            has_gpu: g.is_some(),
            gpu_temp_c: wire_opt(g.and_then(|x| x.temp_c)),
            gpu_power_w: wire_opt(g.and_then(|x| x.power_w)),
            gpu_avg_power_w: wire_opt(g.and_then(|x| x.avg_power_w)),
            gpu_power_limit_w: wire_opt(g.and_then(|x| x.power_limit_w)),
            gpu_usage_pct: wire_opt(g.and_then(|x| x.usage_pct)),
            gpu_clock_mhz: wire_opt(g.and_then(|x| x.clock_mhz)),
            gpu_clock_mem_mhz: wire_opt(g.and_then(|x| x.clock_mem_mhz)),
            vram_used_mib: wire_opt(g.and_then(|x| x.vram_used_mib)),
            vram_total_mib: wire_opt(g.and_then(|x| x.vram_total_mib)),
            gpu_voltage_v: wire_opt(g.and_then(|x| x.voltage_v)),
            vram_temp_c: wire_opt(g.and_then(|x| x.vram_temp_c)),
            hotspot_temp_c: wire_opt(g.and_then(|x| x.hotspot_temp_c)),
            cpu_package_w: wire_opt(s.cpu.package_w),
            cpu_tctl_c: wire_opt(s.cpu.tctl_c),
            ram_used_mib: wire_opt(s.memory.used_mib),
            ram_total_mib: wire_opt(s.memory.total_mib),
            board_v12_v: wire_opt(s.board.v12_v),
            board_v5_v: wire_opt(s.board.v5_v),
            board_v3_3_v: wire_opt(s.board.v3_3_v),
            board_vcore_v: wire_opt(s.board.vcore_v),
            board_soc_v: wire_opt(s.board.soc_v),
            board_dram_v: wire_opt(s.board.dram_v),
            board_vrm_mos_c: wire_opt(s.board.vrm_mos_c),
            board_cpu_socket_c: wire_opt(s.board.cpu_socket_c),
            board_system_c: wire_opt(s.board.system_c),
            board_pch_c: wire_opt(s.board.pch_c),
            board_pcie_x1_c: wire_opt(s.board.pcie_x1_c),
            board_cpu_fan_rpm: wire_opt(s.board.cpu_fan_rpm),
            board_sys_fan1_rpm: wire_opt(s.board.sys_fan1_rpm),
            board_sys_fan3_rpm: wire_opt(s.board.sys_fan3_rpm),
            cpus: s.cpus.iter().map(WireCpuSample::from).collect(),
            gpus: s.gpus.iter().map(WireGpuSample::from).collect(),
        }
    }
}

impl From<WireSample> for Sample {
    fn from(w: WireSample) -> Self {
        let gpu = w.has_gpu.then(|| GpuSample {
            temp_c: opt_wire(w.gpu_temp_c),
            power_w: opt_wire(w.gpu_power_w),
            avg_power_w: opt_wire(w.gpu_avg_power_w),
            power_limit_w: opt_wire(w.gpu_power_limit_w),
            usage_pct: opt_wire(w.gpu_usage_pct),
            clock_mhz: opt_wire(w.gpu_clock_mhz),
            clock_mem_mhz: opt_wire(w.gpu_clock_mem_mhz),
            vram_used_mib: opt_wire(w.vram_used_mib),
            vram_total_mib: opt_wire(w.vram_total_mib),
            voltage_v: opt_wire(w.gpu_voltage_v),
            vram_temp_c: opt_wire(w.vram_temp_c),
            hotspot_temp_c: opt_wire(w.hotspot_temp_c),
        });
        Self {
            timestamp_ms: w.timestamp_ms,
            origin: SourceOrigin::Daemon,
            caps: w.caps.into(),
            gpu,
            cpu: CpuSample {
                package_w: opt_wire(w.cpu_package_w),
                tctl_c: opt_wire(w.cpu_tctl_c),
            },
            cpus: w.cpus.into_iter().map(CpuSample::from).collect(),
            gpus: w.gpus.into_iter().map(GpuSample::from).collect(),
            memory: MemorySample {
                used_mib: opt_wire(w.ram_used_mib),
                total_mib: opt_wire(w.ram_total_mib),
            },
            board: BoardSample {
                v12_v: opt_wire(w.board_v12_v),
                v5_v: opt_wire(w.board_v5_v),
                v3_3_v: opt_wire(w.board_v3_3_v),
                vcore_v: opt_wire(w.board_vcore_v),
                soc_v: opt_wire(w.board_soc_v),
                dram_v: opt_wire(w.board_dram_v),
                vrm_mos_c: opt_wire(w.board_vrm_mos_c),
                cpu_socket_c: opt_wire(w.board_cpu_socket_c),
                system_c: opt_wire(w.board_system_c),
                pch_c: opt_wire(w.board_pch_c),
                pcie_x1_c: opt_wire(w.board_pcie_x1_c),
                cpu_fan_rpm: opt_wire(w.board_cpu_fan_rpm),
                sys_fan1_rpm: opt_wire(w.board_sys_fan1_rpm),
                sys_fan3_rpm: opt_wire(w.board_sys_fan3_rpm),
            },
        }
    }
}

/// D-Bus client proxy for `io.github.abnermsaconi.HwWatcher1`.
#[zbus::proxy(
    interface = "io.github.abnermsaconi.HwWatcher1",
    default_service = "io.github.abnermsaconi.HwWatcher",
    default_path = "/io/github/abnermsaconi/HwWatcher"
)]
pub trait HwWatcher {
    /// Returns the latest telemetry sample.
    fn sample(&self) -> zbus::Result<WireSample>;

    /// Returns the active daemon capabilities.
    fn capabilities(&self) -> zbus::Result<WireCapabilities>;

    /// Broadcasts newly sampled telemetry (2 Hz).
    #[zbus(signal)]
    fn new_sample(&self, sample: WireSample) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests;
