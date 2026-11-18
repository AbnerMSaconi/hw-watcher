// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

use std::path::PathBuf;

use hw_watcher_core::{
    Bar0Source, BoardSample, BoardSource, Clock, CpuPowerSource, GpuSample, GpuSource,
    MemorySample, MemorySource, SourceOrigin, TelemetrySource, VoltageSource,
};
use hw_watcher_sensors::{ProcMemInfoSource, SensorSuite, TelemetryCollector};

struct FakeClock(u64);
impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}

struct FakeGpu(Option<GpuSample>);
impl GpuSource for FakeGpu {
    fn read(&mut self) -> Option<GpuSample> {
        self.0
    }
}

struct FakeCpuPower(Option<f64>);
impl CpuPowerSource for FakeCpuPower {
    fn read_package_w(&mut self) -> Option<f64> {
        self.0
    }
}

struct FakeBoard(Option<f64>, BoardSample);
impl BoardSource for FakeBoard {
    fn read(&self) -> (Option<f64>, BoardSample) {
        (self.0, self.1)
    }
}

struct FakeMemory(MemorySample);
impl MemorySource for FakeMemory {
    fn read(&self) -> MemorySample {
        self.0
    }
}

struct FakeBar0(Option<f64>, Option<f64>);
impl Bar0Source for FakeBar0 {
    fn read_temps(&self) -> (Option<f64>, Option<f64>) {
        (self.0, self.1)
    }
}

struct FakeVoltage(Option<f64>);
impl VoltageSource for FakeVoltage {
    fn read_voltage_v(&self) -> Option<f64> {
        self.0
    }
}

#[test]
fn reads_captured_proc_meminfo_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/proc_meminfo.txt");
    let source = ProcMemInfoSource::at_path(path);
    let mem = source.read();
    assert!(mem.total_mib.unwrap() > 1000.0);
    assert!(mem.used_mib.unwrap() > 0.0);
    assert!(mem.used_pct().unwrap() > 0.0);
}

#[test]
fn unprivileged_collector_hides_missing_root_sensors() {
    let mut collector = TelemetryCollector::<_, _, FakeCpuPower, _, _, FakeBar0, FakeVoltage>::new(
        FakeClock(42_000),
        SourceOrigin::Local,
        SensorSuite {
            gpu: Some(FakeGpu(Some(GpuSample {
                temp_c: Some(40.0),
                power_w: Some(50.0),
                ..Default::default()
            }))),
            cpu_power: None,
            board: FakeBoard(
                Some(55.0),
                BoardSample {
                    v12_v: Some(12.0),
                    ..Default::default()
                },
            ),
            memory: FakeMemory(MemorySample {
                used_mib: Some(4096.0),
                total_mib: Some(16384.0),
            }),
            bar0: None,
            voltage: None,
        },
    );
    let sample = collector.sample();
    assert_eq!(sample.timestamp_ms, 42_000);
    assert!(!sample.caps.rapl);
    assert!(!sample.caps.bar0);
    assert!(!sample.caps.nvapi_voltage);
    assert_eq!(sample.total_power_w(), None);
}

#[test]
fn elevated_collector_merges_bar0_nvapi_and_rapl() {
    let mut collector = TelemetryCollector::new(
        FakeClock(99_000),
        SourceOrigin::Daemon,
        SensorSuite {
            gpu: Some(FakeGpu(Some(GpuSample {
                temp_c: Some(45.0),
                power_w: Some(200.0),
                ..Default::default()
            }))),
            cpu_power: Some(FakeCpuPower(Some(80.0))),
            board: FakeBoard(Some(60.0), BoardSample::default()),
            memory: FakeMemory(MemorySample::default()),
            bar0: Some(FakeBar0(Some(68.0), Some(62.0))),
            voltage: Some(FakeVoltage(Some(0.875))),
        },
    );
    let sample = collector.sample();
    assert!(sample.caps.rapl);
    assert!(sample.caps.bar0);
    assert!(sample.caps.nvapi_voltage);
    assert_eq!(sample.total_power_w(), Some(280.0));
    let gpu = sample.gpu.unwrap();
    assert_eq!(gpu.vram_temp_c, Some(68.0));
    assert_eq!(gpu.hotspot_temp_c, Some(62.0));
    assert_eq!(gpu.voltage_v, Some(0.875));
}
