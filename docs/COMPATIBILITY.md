# Compatibility

**Verified** means checked on real hardware by the author. **Untested** means implemented and unit
tested against fake `/sys` trees, but never run on the real thing. Reports in either direction are
welcome: run `scripts/hw-report.sh` and open a hardware issue.

The only machine this has run on: Linux 7.x, KDE Plasma on X11, AMD Ryzen CPU, NVIDIA GA102 GPU,
Nuvoton `nct6687` board sensors.

## Platform

| Item | Status | Notes |
|---|---|---|
| Linux x86_64 | Verified | |
| Linux aarch64 | Untested | Built by the release workflow, never run |
| macOS, Windows, BSD | Not supported | Needs `/proc`, `/sys`, X11 and D-Bus |
| Prebuilt binary | glibc 2.35 or newer | Built on Ubuntu 22.04. Older distros: build from source |
| Distributions | One verified | Any distro with the libraries below should work |

Runtime needs: a graphical session, D-Bus, and the common desktop libraries (X11/XCB, fontconfig,
xkbcommon). NVIDIA support loads `libnvidia-ml` at run time, so nothing breaks when it is absent.

## Display server and desktop

| Item | Status | Notes |
|---|---|---|
| X11 | Verified | Full feature set |
| Wayland through XWayland | Untested | Default on Wayland sessions; keeps X11 features |
| Native Wayland (`HW_WATCHER_WAYLAND=1`) | Untested, experimental | Move and drag work in theory; saved position, always on top, hide from taskbar and blur are ignored |
| KDE Plasma tray and blur | Verified | Blur uses `_KDE_NET_WM_BLUR_BEHIND_REGION` (KWin only) |
| GNOME | Untested | Tray needs the AppIndicator extension; no blur |
| XFCE, Cinnamon, MATE, others | Untested | Tray needs StatusNotifierItem support; no blur |

Without blur support the widget still works; it is only translucent.

## Sensors

| Sensor | Status | Needs |
|---|---|---|
| NVIDIA GPU: temperature, power, clocks, load, VRAM (NVML) | Verified | NVIDIA driver |
| NVIDIA VRAM/hotspot temperature (BAR0) | Ported from a working prototype, not re-verified | Daemon (root). GA102 only (RTX 3080/3090 family) |
| NVIDIA core voltage (NVAPI) | Verified | Daemon (root) |
| AMD GPU (`amdgpu`) | Untested | Nothing |
| Intel GPU (`i915`, `xe`) | Untested | Nothing |
| RAM (`/proc/meminfo`) | Verified | Nothing |
| CPU temperature `k10temp` (AMD) | Verified | Readable sysfs |
| CPU temperature `coretemp` (Intel), `zenpower` | Untested | Readable sysfs |
| CPU package power (RAPL) | Verified (AMD) | Daemon (root). Intel should work the same |
| Board `nct6687`: voltages, temperatures, fans | Verified | Readable sysfs |
| Board `nct67xx`, `it87x`: temperatures and fans | Untested | Readable sysfs; voltages are not mapped |
| Other GPUs, laptops' embedded controllers, disks | Not supported | |

Unavailable sensors are hidden, never shown as errors.

## Services

| Item | Status | Notes |
|---|---|---|
| systemd units (`service install`) | Untested | Needs systemd; without it, run the app directly |
| D-Bus daemon discovery | Verified on the session bus | System bus by default, `--session` for the session bus |

## Versioning

HW Watcher follows semantic versioning, and the `0.x` series may change settings keys and the D-Bus
interface between minor versions. See `CHANGELOG.md`.
