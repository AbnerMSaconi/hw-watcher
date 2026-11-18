// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! `/proc/meminfo` system RAM reader.

use std::fs;
use std::path::{Path, PathBuf};

use hw_watcher_core::{MemorySample, MemorySource, parse_meminfo};

const DEFAULT_MEMINFO_PATH: &str = "/proc/meminfo";

/// Reads system RAM usage from `/proc/meminfo`.
#[derive(Debug, Clone)]
pub struct ProcMemInfoSource {
    path: PathBuf,
}

impl ProcMemInfoSource {
    /// Opens the standard `/proc/meminfo` source.
    pub fn new() -> Self {
        Self::at_path(PathBuf::from(DEFAULT_MEMINFO_PATH))
    }

    /// Creates a reader targeting a custom path (useful for fixture tests).
    pub fn at_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns `true` if the target file is currently readable.
    pub fn is_readable(&self) -> bool {
        Path::new(&self.path)
            .metadata()
            .is_ok_and(|_| fs::read_to_string(&self.path).is_ok())
    }
}

impl Default for ProcMemInfoSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySource for ProcMemInfoSource {
    fn read(&self) -> MemorySample {
        fs::read_to_string(&self.path)
            .map(|raw| parse_meminfo(&raw))
            .unwrap_or_default()
    }
}
