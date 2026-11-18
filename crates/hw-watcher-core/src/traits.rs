// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Pure domain traits defining sensor, clock, and translation boundaries.

use std::borrow::Cow;

use thiserror::Error;

use crate::model::{BoardSample, GpuSample, MemorySample, Sample};

/// Errors emitted by telemetry adapters.
#[derive(Debug, Error)]
pub enum SensorError {
    /// Hardware or sysfs endpoint could not be opened or read.
    #[error("sensor unavailable: {0}")]
    Unavailable(String),
}

/// Wall-clock timestamp provider for deterministic tests.
pub trait Clock: Send + Sync {
    /// Current wall-clock time in Unix milliseconds.
    fn now_ms(&self) -> u64;
}

/// Key-value translator with `{name}` placeholder substitution.
pub trait Translator: Send + Sync {
    /// Translates `key` into the active UI language.
    fn translate<'a>(&self, key: &'a str) -> Cow<'a, str>;

    /// Translates `key` and replaces `{name}` placeholders with `args`.
    fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut out = self.translate(key).into_owned();
        for (name, value) in args {
            out = out.replace(&format!("{{{name}}}"), value);
        }
        out
    }
}

/// Identity translator that preserves English source strings unchanged.
#[derive(Debug, Default, Clone, Copy)]
pub struct IdentityTranslator;

impl Translator for IdentityTranslator {
    fn translate<'a>(&self, key: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(key)
    }
}

/// GPU telemetry source (typically NVML).
pub trait GpuSource: Send {
    /// Reads current GPU metrics, or `None` if no GPU responds.
    fn read(&mut self) -> Option<GpuSample>;
}

/// CPU package power source (typically RAPL).
pub trait CpuPowerSource: Send {
    /// Reads average package power since the previous call, or `None` if unavailable.
    fn read_package_w(&mut self) -> Option<f64>;
}

/// CPU Tctl and motherboard `hwmon` telemetry source.
pub trait BoardSource: Send {
    /// Reads `(cpu_tctl_c, board_sample)`.
    fn read(&self) -> (Option<f64>, BoardSample);
}

/// System RAM usage source (typically `/proc/meminfo`).
pub trait MemorySource: Send {
    /// Reads current system RAM usage.
    fn read(&self) -> MemorySample;
}

/// Elevated BAR0 VRAM and hotspot temperature source.
pub trait Bar0Source: Send {
    /// Reads `(vram_temp_c, hotspot_temp_c)`.
    fn read_temps(&self) -> (Option<f64>, Option<f64>);
}

/// GPU core voltage source (typically NVAPI).
pub trait VoltageSource: Send {
    /// Reads core voltage in volts, or `None` if unavailable.
    fn read_voltage_v(&self) -> Option<f64>;
}

/// Top-level telemetry stream abstraction (implemented by local, D-Bus, and simulated sources).
pub trait TelemetrySource: Send {
    /// Captures one [`Sample`] snapshot.
    fn sample(&mut self) -> Sample;
}
