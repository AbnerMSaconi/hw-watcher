// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! `hw-watcher` — single-binary multi-mode telemetry panel, daemon, and CLI.

#![deny(unsafe_code)]

mod about;
mod app;
mod app_commands;
mod cli;
mod config;
mod daemon;
mod host;
mod i18n;
mod service;
mod settings_state;
mod settings_sync;
mod settings_ui;
mod source;
mod theme;
mod tray;
mod tray_icon;
mod tray_slots;
mod view_footer;
mod view_model;
mod x11_window;

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use hw_watcher_core::Sample;

use crate::cli::{Cli, Command, FollowArgs, PanelArgs, ServiceSubcmd, SourceArgs, SourceFlags};
use crate::service::ServiceAction;
use crate::source::{SourceMode, TelemetryClient};

const MIB_PER_GIB: f64 = 1024.0;

fn main() {
    init_tracing();
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        eprintln!("hw-watcher: {e:#}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli
        .command
        .unwrap_or_else(|| Command::Panel(PanelArgs::default()))
    {
        Command::Panel(args) => {
            app::run_panel(args.tray, args.settings, resolve_mode(&args.source))
        }
        Command::About => {
            println!("{}", about::full_notice());
            Ok(())
        }
        Command::Daemon(args) => run_async(daemon::run_daemon(args.session, args.simulate)),
        Command::Sample(args) => run_async(run_sample(args)),
        Command::Follow(args) => run_async(run_follow(args)),
        Command::Service(args) => {
            let msg = match args.action {
                ServiceSubcmd::Install { system } => {
                    service::run_service_cmd(ServiceAction::Install, system)?
                }
                ServiceSubcmd::Uninstall { system } => {
                    service::run_service_cmd(ServiceAction::Uninstall, system)?
                }
                ServiceSubcmd::Status { system } => {
                    service::run_service_cmd(ServiceAction::Status, system)?
                }
            };
            if let Some(m) = msg {
                println!("{m}");
            }
            Ok(())
        }
    }
}

fn resolve_mode(flags: &SourceFlags) -> SourceMode {
    if flags.simulate {
        SourceMode::Simulated {
            elevated: flags.elevated,
            multi: flags.simulate_multi,
        }
    } else if flags.local {
        SourceMode::Local
    } else {
        SourceMode::Auto {
            session: flags.session,
        }
    }
}

fn run_async<F>(fut: F) -> Result<()>
where
    F: std::future::Future<Output = Result<()>>,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(fut)
}

async fn run_sample(args: SourceArgs) -> Result<()> {
    let mut client = TelemetryClient::open_full(resolve_mode(&args.source)).await;
    let sample = client.next_sample().await;
    print_sample(&sample, args.json)
}

async fn run_follow(args: FollowArgs) -> Result<()> {
    let mut client = TelemetryClient::open_full(resolve_mode(&args.source)).await;
    let hz = args.hz.clamp(1, 10);
    let mut period = tokio::time::interval(Duration::from_millis(1000 / hz));
    period.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = period.tick() => {
                let sample = client.next_sample().await;
                print_sample(&sample, args.json)?;
            }
        }
    }
    Ok(())
}

fn print_sample(sample: &Sample, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(sample)?);
        return Ok(());
    }
    println!("{}", format_sample_line(sample));
    Ok(())
}

fn format_sample_line(sample: &Sample) -> String {
    let mut parts = Vec::new();
    let cpus = sample.effective_cpus();
    for (idx, c) in cpus.iter().enumerate() {
        let tag = if cpus.len() > 1 {
            format!("CPU {idx}")
        } else {
            "CPU".into()
        };
        if let Some(t) = c.tctl_c {
            parts.push(format!("{tag} {t:.0}°C"));
        }
        if let Some(w) = c.package_w {
            parts.push(format!("{tag} {w:.1}W"));
        }
    }
    let gpus = sample.effective_gpus();
    for (idx, g) in gpus.iter().enumerate() {
        let tag = if gpus.len() > 1 {
            format!("GPU {idx}")
        } else {
            "GPU".into()
        };
        if let Some(t) = g.temp_c {
            parts.push(format!("{tag} {t:.0}°C"));
        }
        if let Some(w) = g.power_w {
            parts.push(format!("{tag} {w:.1}W"));
        }
        if let (Some(u), Some(tot)) = (g.vram_used_mib, g.vram_total_mib) {
            let vtag = if gpus.len() > 1 {
                format!("VRAM {idx}")
            } else {
                "VRAM".into()
            };
            parts.push(format!(
                "{vtag} {:.1}/{:.1}GiB",
                u / MIB_PER_GIB,
                tot / MIB_PER_GIB
            ));
        }
        if let Some(v) = g.vram_temp_c {
            parts.push(format!("VRAM {v:.0}°C"));
        }
        if let Some(h) = g.hotspot_temp_c {
            parts.push(format!("Hotspot {h:.0}°C"));
        }
    }
    if let (Some(u), Some(tot)) = (sample.memory.used_mib, sample.memory.total_mib) {
        parts.push(format!(
            "RAM {:.1}/{:.1}GiB",
            u / MIB_PER_GIB,
            tot / MIB_PER_GIB
        ));
    }
    if let Some(total_w) = sample.total_power_w() {
        parts.push(format!("Total {total_w:.1}W"));
    }
    parts.join(" | ")
}
