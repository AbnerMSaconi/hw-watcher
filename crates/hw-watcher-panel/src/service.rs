// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Optional self-contained installer for systemd system/user units (`hw-watcher service`).
//!
//! Running `hw-watcher` never requires installation; this module only sets up background
//! auto-start or elevated sensor capabilities when explicitly requested.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

const BIN_DEST: &str = "/usr/local/bin/hw-watcher";
const SYSTEM_UNIT: &str = "/etc/systemd/system/hw-watcher.service";
const DBUS_POLICY: &str = "/etc/dbus-1/system.d/io.github.abnermsaconi.HwWatcher.conf";

const UNIT_SYSTEM_TXT: &str = include_str!("../../../packaging/systemd/hw-watcher.service");
const UNIT_USER_TXT: &str = include_str!("../../../packaging/systemd/hw-watcher-user.service");
const DBUS_POLICY_TXT: &str =
    include_str!("../../../packaging/dbus/io.github.abnermsaconi.HwWatcher.conf");
const DESKTOP_TXT: &str =
    include_str!("../../../packaging/desktop/io.github.abnermsaconi.HwWatcher.desktop");

/// Service installation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAction {
    /// Install service files and enable the unit.
    Install,
    /// Stop, disable, and remove service files.
    Uninstall,
    /// Show unit status (`systemctl status`).
    Status,
}

/// Executes `service <install|uninstall|status> [--system | --user]`.
pub fn run_service_cmd(action: ServiceAction, system: bool) -> Result<Option<String>> {
    if system {
        run_system_service(action)
    } else {
        run_user_service(action)
    }
}

fn run_system_service(action: ServiceAction) -> Result<Option<String>> {
    match action {
        ServiceAction::Status => {
            let _ = Command::new("systemctl")
                .args(["status", "--no-pager", "hw-watcher.service"])
                .status();
            Ok(None)
        }
        ServiceAction::Install => {
            let exe = env::current_exe().context("locating current binary")?;
            fs::copy(&exe, BIN_DEST).with_context(|| format!("copying binary to {BIN_DEST}"))?;
            write_root_file(Path::new(SYSTEM_UNIT), UNIT_SYSTEM_TXT)?;
            write_root_file(Path::new(DBUS_POLICY), DBUS_POLICY_TXT)?;
            run_checked("systemctl", &["daemon-reload"])?;
            let _ = Command::new("systemctl")
                .args(["reload", "dbus.service"])
                .status();
            run_checked("systemctl", &["enable", "--now", "hw-watcher.service"])?;
            Ok(Some(format!(
                "Installed system daemon at {BIN_DEST} and started hw-watcher.service."
            )))
        }
        ServiceAction::Uninstall => {
            let _ = Command::new("systemctl")
                .args(["disable", "--now", "hw-watcher.service"])
                .status();
            let _ = fs::remove_file(SYSTEM_UNIT);
            let _ = fs::remove_file(DBUS_POLICY);
            let _ = fs::remove_file(BIN_DEST);
            let _ = Command::new("systemctl").args(["daemon-reload"]).status();
            Ok(Some("Removed system daemon and unit files.".into()))
        }
    }
}

fn run_user_service(action: ServiceAction) -> Result<Option<String>> {
    let unit_path = user_unit_path()?;
    let desktop_path = user_autostart_path()?;
    match action {
        ServiceAction::Status => {
            let _ = Command::new("systemctl")
                .args(["--user", "status", "--no-pager", "hw-watcher.service"])
                .status();
            Ok(None)
        }
        ServiceAction::Install => {
            let exe = env::current_exe().context("locating current binary")?;
            let exe_str = exe.to_string_lossy();
            let unit = UNIT_USER_TXT.replace("/usr/local/bin/hw-watcher", &exe_str);
            let desktop = DESKTOP_TXT.replace("/usr/local/bin/hw-watcher", &exe_str);
            write_user_file(&unit_path, &unit)?;
            write_user_file(&desktop_path, &desktop)?;
            run_checked("systemctl", &["--user", "daemon-reload"])?;
            run_checked(
                "systemctl",
                &["--user", "enable", "--now", "hw-watcher.service"],
            )?;
            Ok(Some(format!(
                "Installed user service ({}) and autostart ({}).",
                unit_path.display(),
                desktop_path.display()
            )))
        }
        ServiceAction::Uninstall => {
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "--now", "hw-watcher.service"])
                .status();
            let _ = fs::remove_file(&unit_path);
            let _ = fs::remove_file(&desktop_path);
            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .status();
            Ok(Some("Removed user service and autostart entry.".into()))
        }
    }
}

fn user_unit_path() -> Result<PathBuf> {
    let base = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .context("XDG_CONFIG_HOME and HOME are unset")?;
    Ok(base.join("systemd/user/hw-watcher.service"))
}

fn user_autostart_path() -> Result<PathBuf> {
    let base = env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .context("XDG_CONFIG_HOME and HOME are unset")?;
    Ok(base.join("autostart/hw-watcher.desktop"))
}

fn write_root_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
        .with_context(|| format!("writing {} (run with sudo for --system)", path.display()))
}

fn write_user_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

fn run_checked(bin: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(bin)
        .args(args)
        .status()
        .with_context(|| format!("executing {bin}"))?;
    if !status.success() {
        bail!("{bin} {} exited with {status}", args.join(" "));
    }
    Ok(())
}
