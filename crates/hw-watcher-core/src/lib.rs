// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Pure domain model, sensor traits, parsers, alert thresholds, and modular tray formatting.

#![forbid(unsafe_code)]

pub mod model;
pub mod parsers;
pub mod thresholds;
pub mod traits;
pub mod tray_format;

pub use model::{
    BoardSample, Capabilities, CpuSample, GpuSample, MemorySample, Sample, SourceOrigin,
};
pub use parsers::{compute_rapl_power_w, decode_bar0_hotspot_c, decode_bar0_vram_c, parse_meminfo};
pub use thresholds::{Band, classify_sample_metric};
pub use traits::{
    Bar0Source, BoardSource, Clock, CpuPowerSource, GpuSource, IdentityTranslator, MemorySource,
    SensorError, TelemetrySource, Translator, VoltageSource,
};
pub use tray_format::{
    TooltipSections, TrayMetric, TrayRendering, TraySlotRendering, build_tray_slots,
    format_tooltip_lines, format_tooltip_lines_for, format_tray_rendering,
};
