// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Sensor aggregation (`TelemetryCollector`) and live multi-device `LocalSensors`.

use std::time::{SystemTime, UNIX_EPOCH};

use hw_watcher_core::{
    Bar0Source, BoardSource, Capabilities, Clock, CpuPowerSource, CpuSample, GpuSample, GpuSource,
    MemorySource, Sample, SourceOrigin, TelemetrySource, VoltageSource,
};
use tracing::info;

use crate::bar0::Bar0TempSensor;
use crate::drm_gpu::SysfsDrmGpuSource;
use crate::hwmon::SysfsHwmonSource;
use crate::meminfo::ProcMemInfoSource;
use crate::nvapi::NvApiVoltageSensor;
use crate::nvml::NvmlGpuSource;
use crate::rapl::SysfsRaplSource;

/// Wall-clock source backed by `std::time::SystemTime`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

/// Injected sensor suite passed to [`TelemetryCollector::new`].
pub struct SensorSuite<G, P, B, M, R, V> {
    /// GPU telemetry source.
    pub gpu: Option<G>,
    /// CPU package power source.
    pub cpu_power: Option<P>,
    /// CPU temperature and motherboard `hwmon` source.
    pub board: B,
    /// System RAM source.
    pub memory: M,
    /// Elevated BAR0 temperature source.
    pub bar0: Option<R>,
    /// Optional NVAPI voltage source.
    pub voltage: Option<V>,
}

/// Composes independent sensor traits into unified [`Sample`] snapshots.
pub struct TelemetryCollector<C, G, P, B, M, R, V> {
    clock: C,
    origin: SourceOrigin,
    sensors: SensorSuite<G, P, B, M, R, V>,
}

impl<C, G, P, B, M, R, V> TelemetryCollector<C, G, P, B, M, R, V> {
    /// Builds a collector from injected sensor implementations.
    pub fn new(clock: C, origin: SourceOrigin, sensors: SensorSuite<G, P, B, M, R, V>) -> Self {
        Self {
            clock,
            origin,
            sensors,
        }
    }
}

impl<C, G, P, B, M, R, V> TelemetrySource for TelemetryCollector<C, G, P, B, M, R, V>
where
    C: Clock,
    G: GpuSource,
    P: CpuPowerSource,
    B: BoardSource,
    M: MemorySource,
    R: Bar0Source,
    V: VoltageSource,
{
    fn sample(&mut self) -> Sample {
        let timestamp_ms = self.clock.now_ms();
        let mut gpu = self.sensors.gpu.as_mut().and_then(GpuSource::read);
        let (vram_c, hotspot_c) = self
            .sensors
            .bar0
            .as_ref()
            .map_or((None, None), Bar0Source::read_temps);
        let voltage_v = self
            .sensors
            .voltage
            .as_ref()
            .and_then(VoltageSource::read_voltage_v);
        if let Some(ref mut g) = gpu {
            g.vram_temp_c = vram_c;
            g.hotspot_temp_c = hotspot_c;
            g.voltage_v = voltage_v;
        }

        let package_w = self
            .sensors
            .cpu_power
            .as_mut()
            .and_then(CpuPowerSource::read_package_w);
        let (tctl_c, board) = self.sensors.board.read();
        let memory = self.sensors.memory.read();
        let cpu = CpuSample { package_w, tctl_c };
        let cpus = if package_w.is_some() || tctl_c.is_some() {
            vec![cpu]
        } else {
            Vec::new()
        };
        let gpus = gpu.into_iter().collect();
        let caps = Capabilities {
            nvml: gpu.is_some(),
            rapl: package_w.is_some(),
            hwmon_cpu: tctl_c.is_some(),
            hwmon_board: board_present(&board),
            meminfo: memory.total_mib.is_some(),
            bar0: vram_c.is_some() || hotspot_c.is_some(),
            nvapi_voltage: voltage_v.is_some(),
        };
        Sample {
            timestamp_ms,
            origin: self.origin,
            caps,
            gpu,
            cpu,
            cpus,
            gpus,
            memory,
            board,
        }
    }
}

fn board_present(b: &hw_watcher_core::BoardSample) -> bool {
    b.v12_v.is_some() || b.vrm_mos_c.is_some() || b.system_c.is_some() || b.cpu_fan_rpm.is_some()
}

/// Live OS/hardware telemetry collector supporting multiple CPUs and GPUs, working unprivileged
/// and automatically picking up elevated sensors (RAPL, BAR0, NVAPI) when readable.
pub struct LocalSensors {
    gpu: Option<NvmlGpuSource>,
    drm_gpus: Option<SysfsDrmGpuSource>,
    bar0s: Vec<Option<Bar0TempSensor>>,
    voltages: Vec<Option<NvApiVoltageSensor>>,
    cpu_power: Option<SysfsRaplSource>,
    board: SysfsHwmonSource,
    memory: ProcMemInfoSource,
}

impl LocalSensors {
    /// Probes all local sensors (including NVAPI voltage) and logs active capabilities.
    pub fn open() -> Self {
        Self::open_with_nvapi(true)
    }

    /// Probes local sensors without loading `libnvidia-api.so.1` (`libcuda.so`), saving ~16 MB RAM.
    pub fn open_lightweight() -> Self {
        Self::open_with_nvapi(false)
    }

    fn open_with_nvapi(with_nvapi: bool) -> Self {
        let gpu = NvmlGpuSource::open();
        let identities = gpu.as_ref().map_or(&[][..], NvmlGpuSource::identities);
        let bar0s: Vec<Option<Bar0TempSensor>> = identities
            .iter()
            .map(|id| Bar0TempSensor::open(&id.pci, id.pci_device_id))
            .collect();
        let voltages: Vec<Option<NvApiVoltageSensor>> = identities
            .iter()
            .map(|id| {
                with_nvapi
                    .then(|| NvApiVoltageSensor::open(&id.pci))
                    .flatten()
            })
            .collect();
        let drm_gpus = SysfsDrmGpuSource::open();
        let cpu_power = SysfsRaplSource::open();
        let board = SysfsHwmonSource::open();
        let memory = ProcMemInfoSource::new();

        info!(
            nvml_gpus = identities.len(),
            drm_gpus = drm_gpus.as_ref().map_or(0, SysfsDrmGpuSource::len),
            rapl = cpu_power.is_some(),
            bar0 = bar0s.iter().any(Option::is_some),
            nvapi = voltages.iter().any(Option::is_some),
            hwmon_cpu = board.has_cpu(),
            hwmon_board = board.has_board(),
            meminfo = memory.is_readable(),
            "local telemetry initialized"
        );

        Self {
            gpu,
            drm_gpus,
            bar0s,
            voltages,
            cpu_power,
            board,
            memory,
        }
    }
}

impl TelemetrySource for LocalSensors {
    fn sample(&mut self) -> Sample {
        let timestamp_ms = SystemClock.now_ms();
        let mut gpus: Vec<GpuSample> = self
            .gpu
            .as_mut()
            .map(NvmlGpuSource::read_all)
            .unwrap_or_default();
        // BAR0/NVAPI readers are indexed by NVML order, so AMD/Intel GPUs are appended after.
        if let Some(drm) = self.drm_gpus.as_mut() {
            gpus.extend(drm.read_all());
        }

        let mut any_bar0 = false;
        let mut any_nvapi = false;
        for (idx, g) in gpus.iter_mut().enumerate() {
            if let Some(Some(b)) = self.bar0s.get(idx) {
                let (vram_c, hotspot_c) = b.read_temps();
                g.vram_temp_c = vram_c;
                g.hotspot_temp_c = hotspot_c;
                any_bar0 |= vram_c.is_some() || hotspot_c.is_some();
            }
            if let Some(Some(v)) = self.voltages.get(idx) {
                g.voltage_v = v.read_voltage_v();
                any_nvapi |= g.voltage_v.is_some();
            }
        }

        let rapl_list = self
            .cpu_power
            .as_mut()
            .map(SysfsRaplSource::read_all_packages_w)
            .unwrap_or_default();
        let hwmon_temps = self.board.read_all_cpu_temps();
        let (_, board) = self.board.read();
        let memory = self.memory.read();

        let cpu_count = rapl_list.len().max(hwmon_temps.len());
        let cpus: Vec<CpuSample> = (0..cpu_count)
            .map(|i| CpuSample {
                package_w: rapl_list.get(i).copied().flatten(),
                tctl_c: hwmon_temps.get(i).copied().flatten(),
            })
            .filter(|c| c.package_w.is_some() || c.tctl_c.is_some())
            .collect();

        let agg_pkg = {
            let mut vals = cpus.iter().filter_map(|c| c.package_w);
            vals.next().map(|f| vals.fold(f, |a, b| a + b))
        };
        let agg_temp = cpus.iter().filter_map(|c| c.tctl_c).reduce(f64::max);
        let cpu = CpuSample {
            package_w: agg_pkg,
            tctl_c: agg_temp,
        };
        let gpu = gpus.first().copied();

        let caps = Capabilities {
            nvml: !gpus.is_empty(),
            rapl: agg_pkg.is_some(),
            hwmon_cpu: agg_temp.is_some(),
            hwmon_board: board_present(&board),
            meminfo: memory.total_mib.is_some(),
            bar0: any_bar0,
            nvapi_voltage: any_nvapi,
        };

        Sample {
            timestamp_ms,
            origin: SourceOrigin::Local,
            caps,
            gpu,
            cpu,
            cpus,
            gpus,
            memory,
            board,
        }
    }
}
