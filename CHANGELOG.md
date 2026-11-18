# Changelog

## 0.1.0

First public release.

### Features
- Floating translucent widget, modular tray (SNI) badges with a configurable tooltip, and a Control Panel (General, Taskbar, Widget, Appearance tabs).
- Single `hw-watcher` binary (alias `hww`) with `panel`, `daemon`, `sample`, `follow` and `service` modes.
- Sensors: NVIDIA (NVML, plus BAR0 and NVAPI through the daemon), AMD (`amdgpu`) and Intel (`i915` / `xe`) GPUs from DRM sysfs, RAM, CPU temperature (`k10temp`, `zenpower`, `coretemp`), RAPL package power, `nct6687` board sensors and a generic `nct67xx` / `it87x` fallback for temperatures and fans.
- Automatic daemon discovery over D-Bus with a lazy local fallback; unavailable sensors are hidden.
- Themes, per-item colors, opacity and compositor blur (`_KDE_NET_WM_BLUR_BEHIND_REGION`).
- English UI with a bundled Portuguese translation.
- X11 by default (also through XWayland); experimental native Wayland backend with `HW_WATCHER_WAYLAND=1`.
- Release workflow (x86_64 and aarch64 tarballs), CI, install and hardware-report scripts, contribution docs and issue templates.

### Notes
- The AMD, Intel and generic board readers are tested against fake sysfs trees only; they have not been verified on real hardware yet.
- Live readings only: nothing is recorded to disk.
