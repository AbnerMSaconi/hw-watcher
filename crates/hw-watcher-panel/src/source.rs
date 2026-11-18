// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Telemetry client supporting automatic D-Bus daemon discovery with local fallback.

use hw_watcher_core::{Sample, TelemetrySource};
use hw_watcher_proto::{HwWatcherProxy, system_bus_address};
use hw_watcher_sensors::{LocalSensors, SimulatedSensors};
use zbus::Connection;

/// Source selection requested by CLI flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    /// Connect to `hw-watcher daemon` if available; otherwise read local sensors in-process.
    Auto {
        /// Prefer session bus instead of system bus.
        session: bool,
    },
    /// Force in-process unprivileged/local reading (no daemon probe).
    Local,
    /// Generate synthetic samples (`elevated` enables simulated RAPL/BAR0/NVAPI).
    Simulated {
        /// Whether to include simulated elevated sensors.
        elevated: bool,
        /// Simulate 2 CPUs and 2 GPUs.
        multi: bool,
    },
}

/// Active telemetry stream resolved at startup (and able to fall back lazily if daemon drops).
pub enum TelemetryClient {
    /// Connected to `hw-watcher daemon` over D-Bus.
    Daemon {
        /// Live proxy to `io.github.abnermsaconi.HwWatcher1`.
        proxy: HwWatcherProxy<'static>,
        /// Lazily opened local fallback collector if the daemon stops or restarts.
        fallback: Option<Box<LocalSensors>>,
    },
    /// In-process local sensors (or simulated source).
    InProcess(Box<dyn TelemetrySource>),
}

impl TelemetryClient {
    /// Connects according to `mode` using lightweight local sensors (no NVAPI/CUDA) on fallback.
    pub async fn open(mode: SourceMode) -> Self {
        Self::open_with_profile(mode, false).await
    }

    /// Connects according to `mode`, optionally probing NVAPI voltage in local fallback.
    pub async fn open_full(mode: SourceMode) -> Self {
        Self::open_with_profile(mode, true).await
    }

    async fn open_with_profile(mode: SourceMode, full_local: bool) -> Self {
        let open_local = || {
            if full_local {
                LocalSensors::open()
            } else {
                LocalSensors::open_lightweight()
            }
        };
        match mode {
            SourceMode::Simulated { elevated, multi } => {
                let src = if multi {
                    SimulatedSensors::new_multi(elevated, 2, 2)
                } else {
                    SimulatedSensors::new(elevated)
                };
                Self::InProcess(Box::new(src))
            }
            SourceMode::Local => Self::InProcess(Box::new(open_local())),
            SourceMode::Auto { session } => {
                if let Some(proxy) = try_connect_daemon(session).await {
                    return Self::Daemon {
                        proxy,
                        fallback: None,
                    };
                }
                Self::InProcess(Box::new(open_local()))
            }
        }
    }

    /// Fetches one fresh [`Sample`].
    pub async fn next_sample(&mut self) -> Sample {
        match self {
            Self::InProcess(src) => src.sample(),
            Self::Daemon { proxy, fallback } => match proxy.sample().await {
                Ok(wire) => Sample::from(wire),
                Err(_) => fallback
                    .get_or_insert_with(|| Box::new(LocalSensors::open_lightweight()))
                    .sample(),
            },
        }
    }
}

async fn try_connect_daemon(session: bool) -> Option<HwWatcherProxy<'static>> {
    let conn = if session {
        Connection::session().await.ok()?
    } else {
        zbus::conn::Builder::address(system_bus_address().as_str())
            .ok()?
            .build()
            .await
            .ok()?
    };
    let proxy = HwWatcherProxy::new(&conn).await.ok()?;
    proxy.capabilities().await.ok()?;
    Some(proxy)
}
