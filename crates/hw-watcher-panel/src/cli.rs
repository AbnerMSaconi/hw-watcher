// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Single-binary CLI definition for `hw-watcher` modes.

use clap::{Args, Parser, Subcommand};

/// Version string shown by `--version`: crate version plus the build's commit id.
const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (commit ",
    env!("HW_WATCHER_COMMIT"),
    ")"
);

/// Portable hardware telemetry: power, temperature, RAM and VRAM.
#[derive(Debug, Parser)]
#[command(
    name = "hw-watcher",
    version = VERSION,
    about = "Floating CPU/GPU/RAM/VRAM monitor and optional telemetry daemon"
)]
pub struct Cli {
    /// Mode to run (defaults to `panel` when omitted).
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Available execution modes.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show authorship, license and provenance information.
    About,
    /// Floating panel and modular system tray icon (default).
    Panel(PanelArgs),
    /// Headless D-Bus telemetry daemon (no CSV storage).
    Daemon(DaemonArgs),
    /// Print one telemetry snapshot to stdout and exit.
    Sample(SourceArgs),
    /// Stream telemetry snapshots continuously to stdout.
    Follow(FollowArgs),
    /// Optional systemd unit installer/uninstaller (never required to run).
    Service(ServiceArgs),
}

/// Shared telemetry source flags across `panel`, `sample`, and `follow`.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Args, Default)]
pub struct SourceFlags {
    /// Force local in-process sensors (do not connect to daemon).
    #[arg(long)]
    pub local: bool,
    /// Connect to the session D-Bus instead of the system D-Bus.
    #[arg(long)]
    pub session: bool,
    /// Generate synthetic sensor readings for testing.
    #[arg(long)]
    pub simulate: bool,
    /// Include simulated elevated sensors (RAPL, BAR0, NVAPI) with `--simulate`.
    #[arg(long)]
    pub elevated: bool,
    /// Simulate 2 CPUs and 2 GPUs when `--simulate` is active.
    #[arg(long)]
    pub simulate_multi: bool,
}

/// Flags for `hw-watcher panel`.
#[derive(Debug, Clone, Args, Default)]
pub struct PanelArgs {
    /// Start minimized in the system tray.
    #[arg(long)]
    pub tray: bool,
    /// Open the Control Panel window immediately on launch.
    #[arg(long)]
    pub settings: bool,
    /// Telemetry source flags.
    #[command(flatten)]
    pub source: SourceFlags,
}

/// Flags for `hw-watcher daemon`.
#[derive(Debug, Clone, Args)]
pub struct DaemonArgs {
    /// Publish on the session bus instead of the system bus.
    #[arg(long)]
    pub session: bool,
    /// Generate synthetic elevated telemetry for testing.
    #[arg(long)]
    pub simulate: bool,
}

/// Flags for `hw-watcher sample`.
#[derive(Debug, Clone, Args)]
pub struct SourceArgs {
    /// Output snapshot as JSON.
    #[arg(long)]
    pub json: bool,
    /// Telemetry source flags.
    #[command(flatten)]
    pub source: SourceFlags,
}

/// Flags for `hw-watcher follow`.
#[derive(Debug, Clone, Args)]
pub struct FollowArgs {
    /// Polling frequency in Hz (1..=10).
    #[arg(long, default_value_t = 1)]
    pub hz: u64,
    /// Output one JSON line per sample.
    #[arg(long)]
    pub json: bool,
    /// Telemetry source flags.
    #[command(flatten)]
    pub source: SourceFlags,
}

/// Flags for `hw-watcher service`.
#[derive(Debug, Clone, Args)]
pub struct ServiceArgs {
    /// Service action (`install`, `uninstall`, or `status`).
    #[command(subcommand)]
    pub action: ServiceSubcmd,
}

/// Actions supported by `hw-watcher service`.
#[derive(Debug, Clone, Subcommand)]
pub enum ServiceSubcmd {
    /// Install systemd unit and enable automatic start.
    Install {
        /// Install system-wide daemon in `/etc/systemd/system` (requires root; unlocks RAPL/BAR0).
        #[arg(long)]
        system: bool,
    },
    /// Stop, disable, and remove installed unit files.
    Uninstall {
        /// Remove system-wide daemon (requires root).
        #[arg(long)]
        system: bool,
    },
    /// Print systemd unit status.
    Status {
        /// Query system-wide daemon instead of user unit.
        #[arg(long)]
        system: bool,
    },
}
