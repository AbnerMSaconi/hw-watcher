// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Lightweight headless D-Bus telemetry daemon (`hw-watcher daemon`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use hw_watcher_core::{Sample, TelemetrySource};
use hw_watcher_proto::{BUS_NAME, OBJECT_PATH, WireCapabilities, WireSample, system_bus_address};
use hw_watcher_sensors::{LocalSensors, SimulatedSensors};
use tracing::info;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, interface};

const SAMPLE_INTERVAL_MS: u64 = 100;
const SIGNAL_EVERY_TICKS: u64 = 5;

struct DaemonService {
    latest: Arc<Mutex<Sample>>,
}

#[interface(name = "io.github.abnermsaconi.HwWatcher1")]
impl DaemonService {
    fn sample(&self) -> WireSample {
        self.latest.lock().map_or_else(
            |_| WireSample::from(&Sample::default()),
            |s| WireSample::from(&*s),
        )
    }

    fn capabilities(&self) -> WireCapabilities {
        let guard = self.latest.lock();
        guard
            .map(|s| WireCapabilities::from(s.caps))
            .unwrap_or_default()
    }

    #[zbus(signal)]
    async fn new_sample(ctxt: &SignalEmitter<'_>, sample: WireSample) -> zbus::Result<()>;
}

/// Runs the D-Bus daemon until Ctrl+C / SIGTERM.
pub async fn run_daemon(session: bool, simulate: bool) -> Result<()> {
    let mut source: Box<dyn TelemetrySource> = if simulate {
        Box::new(SimulatedSensors::new(true))
    } else {
        Box::new(LocalSensors::open())
    };
    let initial = source.sample();
    let latest = Arc::new(Mutex::new(initial));

    let conn = connect_bus(session).await?;
    conn.object_server()
        .at(
            OBJECT_PATH,
            DaemonService {
                latest: Arc::clone(&latest),
            },
        )
        .await
        .context("registering D-Bus object")?;
    conn.request_name(BUS_NAME)
        .await
        .context("requesting D-Bus service name")?;

    info!(
        bus = if session { "session" } else { "system" },
        "daemon ready"
    );
    run_sampler_loop(source, latest, conn).await;
    Ok(())
}

async fn connect_bus(session: bool) -> Result<Connection> {
    if session {
        Connection::session()
            .await
            .context("connecting to session D-Bus")
    } else {
        zbus::conn::Builder::address(system_bus_address().as_str())
            .context("building system D-Bus address")?
            .build()
            .await
            .context("connecting to system D-Bus")
    }
}

async fn run_sampler_loop(
    mut source: Box<dyn TelemetrySource>,
    latest: Arc<Mutex<Sample>>,
    conn: Connection,
) {
    let mut period = tokio::time::interval(Duration::from_millis(SAMPLE_INTERVAL_MS));
    period.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut tick: u64 = 0;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("daemon stopping on interrupt");
                break;
            }
            _ = period.tick() => {
                let sample = source.sample();
                if let Ok(mut guard) = latest.lock() {
                    *guard = sample.clone();
                }
                tick = tick.wrapping_add(1);
                if tick.is_multiple_of(SIGNAL_EVERY_TICKS) {
                    emit_sample(&conn, &sample).await;
                }
            }
        }
    }
}

async fn emit_sample(conn: &Connection, sample: &Sample) {
    let Ok(iface) = conn
        .object_server()
        .interface::<_, DaemonService>(OBJECT_PATH)
        .await
    else {
        return;
    };
    let wire = WireSample::from(sample);
    let _ = DaemonService::new_sample(iface.signal_emitter(), wire).await;
}
