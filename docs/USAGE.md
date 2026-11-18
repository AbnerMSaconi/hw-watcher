# Usage

`hw-watcher` is a single binary (short alias: `hww`). Run it with no arguments to get the floating
panel plus tray badges. Nothing needs to be installed and no root is required.

## Install

**Prebuilt (recommended).** Download `hw-watcher-<version>-linux-<arch>.tar.gz` from the Releases
page, verify it and run it:

```bash
sha256sum -c hw-watcher-*.tar.gz.sha256
tar xzf hw-watcher-*-linux-x86_64.tar.gz
cd hw-watcher-*/
./hw-watcher                       # run directly
./scripts/install.sh ./hw-watcher  # optional: ~/.local/bin, launcher entry, autostart, `hww` alias
```

To confirm the download is the official build, also run
`gh attestation verify <tarball> --repo AbnerMSaconi/hw-watcher` (see `PROVENANCE.md`).
`BUILD-INFO.txt` inside the tarball records the version, commit, target and build date. Check your
copy at any time with `hw-watcher --version` (it prints the version and commit).

**From source.** Needs Rust 1.93+ and the usual desktop development libraries
(`fontconfig`, `xkbcommon`, `xcb`/`x11`, `wayland`).

```bash
cargo build --release
./target/release/hw-watcher
```

**Uninstall.** Delete `~/.local/bin/hw-watcher`, `~/.local/bin/hww`,
`~/.local/share/applications/hw-watcher.desktop` and `~/.config/autostart/hw-watcher.desktop`.
Your settings live in `~/.config/hw-watcher/` and can be removed too.

## Modes

| Command | What it does |
|---|---|
| `hw-watcher` / `hw-watcher panel` | Floating widget and tray badges (default) |
| `hw-watcher panel --tray` | Start hidden in the tray |
| `hw-watcher panel --settings` | Open the Control Panel on start |
| `hw-watcher sample [--json]` | Print one reading and exit |
| `hw-watcher follow [--hz N] [--json]` | Stream readings (1 to 10 Hz) |
| `hw-watcher about` | Author, license and provenance information |
| `hw-watcher daemon` | Headless D-Bus daemon for root-only sensors |
| `hw-watcher service install\|uninstall\|status` | Manage the optional systemd units |

Flags shared by `panel`, `sample` and `follow`:

| Flag | Meaning |
|---|---|
| `--local` | Read sensors in-process, ignore any daemon |
| `--session` | Use the session D-Bus instead of the system D-Bus |
| `--simulate` | Fake readings, no hardware needed (add `--elevated` for root-only values, `--simulate-multi` for 2 CPUs and 2 GPUs) |

## The widget and the tray

- Drag the widget to move it. Right-click it, or the tray icon, for the menu: show/hide, always on
  top, hide from taskbar, which metric the badge shows, tooltip sections, Control Panel and Quit.
- The tray shows 0 to 8 badges. Each badge is one metric (total power, GPU temperature, RAM %, ...)
  and the tooltip lists the details.
- Rows and footer lines whose sensor cannot be read are hidden instead of showing blanks.

## Control Panel

Open it from the menu or with `--settings`. Changes apply immediately and are saved.

| Tab | Contents |
|---|---|
| **General** | Update interval (250 ms to 5 s), list multiple CPUs/GPUs separately |
| **Taskbar** | How many badges, which metrics and their order, tooltip sections |
| **Widget** | Show/hide, always on top, hide from taskbar; which rows and details appear |
| **Appearance** | Presets, background/border color and opacity, blur, text and status colors, taskbar badge, per-item label colors |

The footer has **Reset defaults** (keeps the window position) and **Close**.

## Optional daemon (root-only sensors)

CPU package power (RAPL), GPU VRAM/hotspot temperature (BAR0) and core voltage (NVAPI) need elevated
read access. Install the daemon once and the panel finds it automatically:

```bash
sudo hw-watcher service install --system   # system daemon (systemd)
hw-watcher service install                 # user-level units instead, no root, no elevated sensors
hw-watcher service status
sudo hw-watcher service uninstall
```

The system unit is sandboxed and holds only `CAP_DAC_OVERRIDE`, `CAP_DAC_READ_SEARCH` and
`CAP_SYS_RAWIO`. The panel shows its data source in the widget header (`daemon`, `local` or `sim`).

## Configuration

Stored in `$XDG_CONFIG_HOME/hw-watcher/config.toml` (usually `~/.config/hw-watcher/config.toml`).
Use the Control Panel rather than editing it by hand: an unreadable file is ignored and replaced by
defaults on the next save. Main keys: `x`, `y`, `visible`, `always_on_top`, `skip_taskbar`,
`interval_ms`, `separate_cpus`, `separate_gpus`, `tray_count`, `tray_metrics`, and the `[tooltip]`,
`[widget_items]` and `[theme]` tables.

Environment variable: `HW_WATCHER_WAYLAND=1` opts into the experimental native Wayland backend
(see `COMPATIBILITY.md`).

## Troubleshooting

| Problem | Try |
|---|---|
| No tray icon | Your desktop needs a StatusNotifierItem host (GNOME: install an AppIndicator extension) |
| Blur does nothing | Needs KWin with compositing on X11 and widget opacity below 100% |
| A value is missing | The sensor is unsupported or needs root. Run `hw-watcher sample` and `scripts/hw-report.sh`, then open a hardware issue |
| CPU power / VRAM temperature missing | Install the daemon (see above) |
| Widget not where you expect on Wayland | Position and always-on-top are X11 features; use the default XWayland mode |
| High CPU use | Raise the update interval in General (1 s is plenty) |
