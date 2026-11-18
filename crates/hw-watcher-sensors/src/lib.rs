// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Hardware and OS telemetry adapters implementing [`hw_watcher_core`] traits.

#![deny(unsafe_code)]

pub mod bar0;
pub mod collector;
pub mod drm_gpu;
pub mod hwmon;
pub mod meminfo;
pub mod nvapi;
pub mod nvml;
pub mod rapl;
pub mod simulated;

pub use collector::{LocalSensors, SensorSuite, SystemClock, TelemetryCollector};
pub use drm_gpu::SysfsDrmGpuSource;
pub use hwmon::SysfsHwmonSource;
pub use meminfo::ProcMemInfoSource;
pub use nvml::NvmlGpuSource;
pub use rapl::SysfsRaplSource;
pub use simulated::SimulatedSensors;
