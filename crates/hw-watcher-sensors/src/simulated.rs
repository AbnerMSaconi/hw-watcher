// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Deterministic synthetic telemetry source (`--simulate`) with optional multi-CPU/GPU.

use hw_watcher_core::{
    BoardSample, Capabilities, Clock, CpuSample, GpuSample, MemorySample, Sample, SourceOrigin,
    TelemetrySource,
};

use crate::collector::SystemClock;

/// Deterministic synthetic telemetry source for `--simulate` testing without hardware.
pub struct SimulatedSensors {
    step: u64,
    elevated: bool,
    cpu_count: usize,
    gpu_count: usize,
}

impl SimulatedSensors {
    /// Creates a single-CPU, single-GPU synthetic source.
    pub fn new(elevated: bool) -> Self {
        Self::new_multi(elevated, 1, 1)
    }

    /// Creates a synthetic source simulating `cpu_count` CPUs and `gpu_count` GPUs.
    pub fn new_multi(elevated: bool, cpu_count: usize, gpu_count: usize) -> Self {
        Self {
            step: 0,
            elevated,
            cpu_count: cpu_count.max(1),
            gpu_count: gpu_count.max(1),
        }
    }
}

impl TelemetrySource for SimulatedSensors {
    fn sample(&mut self) -> Sample {
        self.step = self.step.wrapping_add(1);
        let wave = ((self.step % 20) as f64) / 19.0;

        let mut cpus = Vec::with_capacity(self.cpu_count);
        for idx in 0..self.cpu_count {
            let offset = (idx as f64) * 4.0;
            cpus.push(CpuSample {
                package_w: self.elevated.then_some(38.0 + wave * 72.0 + offset),
                tctl_c: Some(52.0 + wave * 24.0 + offset),
            });
        }

        let mut gpus = Vec::with_capacity(self.gpu_count);
        for idx in 0..self.gpu_count {
            let offset = (idx as f64) * 5.0;
            let gpu_w = 45.0 + wave * 280.0 - offset * 4.0;
            gpus.push(GpuSample {
                temp_c: Some(38.0 + wave * 34.0 + offset),
                power_w: Some(gpu_w),
                avg_power_w: Some(gpu_w * 0.96),
                power_limit_w: Some(420.0),
                usage_pct: Some((8.0 + wave * 84.0 - offset * 3.0).max(1.0)),
                clock_mhz: Some(210.0 + wave * 1650.0),
                clock_mem_mhz: Some(405.0 + wave * 9000.0),
                vram_used_mib: Some(2048.0 + wave * 10_240.0 + offset * 256.0),
                vram_total_mib: Some(24_576.0),
                voltage_v: self.elevated.then_some(0.735 + wave * 0.25),
                vram_temp_c: self.elevated.then_some(46.0 + wave * 34.0 + offset),
                hotspot_temp_c: self.elevated.then_some(48.0 + wave * 36.0 + offset),
            });
        }

        let cpu = CpuSample {
            package_w: {
                let mut p = cpus.iter().filter_map(|c| c.package_w);
                p.next().map(|f| p.fold(f, |a, b| a + b))
            },
            tctl_c: cpus.iter().filter_map(|c| c.tctl_c).reduce(f64::max),
        };
        let gpu = gpus.first().copied();

        Sample {
            timestamp_ms: SystemClock.now_ms(),
            origin: SourceOrigin::Simulated,
            caps: Capabilities {
                nvml: true,
                rapl: self.elevated,
                hwmon_cpu: true,
                hwmon_board: true,
                meminfo: true,
                bar0: self.elevated,
                nvapi_voltage: self.elevated,
            },
            gpu,
            cpu,
            cpus,
            gpus,
            memory: MemorySample {
                used_mib: Some(9_216.0 + wave * 8_192.0),
                total_mib: Some(32_768.0),
            },
            board: BoardSample {
                v12_v: Some(12.048 - wave * 0.06),
                v5_v: Some(5.012),
                v3_3_v: Some(3.318),
                vcore_v: Some(1.185),
                soc_v: Some(1.045),
                dram_v: Some(1.350),
                vrm_mos_c: Some(44.0 + wave * 8.0),
                cpu_socket_c: Some(41.0 + wave * 7.0),
                system_c: Some(34.0 + wave * 4.0),
                pch_c: Some(47.0 + wave * 5.0),
                pcie_x1_c: Some(36.0 + wave * 3.0),
                cpu_fan_rpm: Some(1250.0 + wave * 500.0),
                sys_fan1_rpm: Some(980.0 + wave * 200.0),
                sys_fan3_rpm: Some(920.0 + wave * 180.0),
            },
        }
    }
}
