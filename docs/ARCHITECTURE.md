# Architecture

`hw-watcher` is structured as a standalone Rust workspace designed to operate independently and to be embedded by other tools as a subservice.

## Crates

| Crate | Safety | Responsibility |
|---|---|---|
| `hw-watcher-core` | `#![forbid(unsafe_code)]` | Pure domain model (`Sample`, `MemorySample`, `GpuSample`, `Capabilities`), sensor traits, RAPL/meminfo/BAR0 decoders, alert thresholds, and modular tray formatting. Zero I/O. |
| `hw-watcher-proto` | `#![forbid(unsafe_code)]` | D-Bus wire contract (`io.github.abnermsaconi.HwWatcher1`) converting `Option<f64>` ↔ `f64::NAN`. |
| `hw-watcher-sensors` | `#![deny(unsafe_code)]` (isolated `bar0` & `nvapi`) | Concrete implementations of `hw-watcher-core` traits (`NvmlGpuSource`, `ProcMemInfoSource`, `SysfsRaplSource`, `SysfsHwmonSource`, `Bar0TempSensor`, `NvApiVoltageSensor`) plus `LocalSensors` and `SimulatedSensors`. |
| `hw-watcher-panel` | `#![forbid(unsafe_code)]` | Single binary (`hw-watcher`) providing the Slint floating UI, modular `ksni` tray, D-Bus `daemon`, CLI (`sample`, `follow`), and optional `service` helper. |

## Dual-Mode Execution ("Executed vs. Installed")

1. **Executed directly (no install)**: `TelemetryClient::open(SourceMode::Auto)` probes D-Bus. When no daemon responds, it runs `LocalSensors` in-process. Sensors denied by permissions (RAPL `0400`, BAR0 `0600`) remain `None`, and both `view_model::build_panel_view` and `tray_format` omit unavailable rows/items cleanly.
2. **Installed (`hw-watcher daemon`)**: The systemd unit grants `CAP_DAC_OVERRIDE`, `CAP_DAC_READ_SEARCH`, and `CAP_SYS_RAWIO` (read-only) to the daemon, unlocking RAPL CPU package W and BAR0 VRAM/hotspot temperatures over D-Bus without making `energy_uj` world-readable.

## Embedding

All telemetry consumers depend only on `hw_watcher_core::TelemetrySource` and `hw_watcher_core::Sample`, allowing other applications to either consume the D-Bus interface or embed `hw-watcher-core` + `hw-watcher-sensors` directly.
