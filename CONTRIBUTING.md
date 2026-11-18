# Contributing

Thanks for helping! Bug reports, hardware support and small fixes are all welcome.

## Quick start

```bash
cargo build --release
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Please keep all three of the last commands clean before opening a pull request.

## Reporting hardware problems

Run `scripts/hw-report.sh` and paste the output in a "Hardware support request" issue. It lists
hwmon chip names, GPU drivers and your session type, and never prints hostnames, serial numbers or
user names.

## Adding or fixing a sensor

Sensors live in `crates/hw-watcher-sensors/src` and implement the small traits in
`crates/hw-watcher-core/src/traits.rs`. The pattern that keeps contributions easy to review:

1. Read from sysfs through a function that takes a **root path** (`open_at(root)`), so tests can
   point it at a fake tree.
2. Return `None` for anything unreadable. The UI hides missing values, so no placeholders.
3. Add a test that builds a fake sysfs tree in a temp directory (see `drm_gpu.rs` and `hwmon.rs`).
   You do not need the real hardware to write the test, only the file names and values from your
   own `/sys` tree.

Good first contributions: voltage-rail mapping for more Super-I/O chips, AMD/Intel GPU fields on
more driver versions, Wayland behavior reports.

## Code conventions

- Source, comments and UI strings are in English; Portuguese lives in
  `crates/hw-watcher-panel/lang/pt/LC_MESSAGES/hw-watcher-panel.po`. New UI text needs an entry there.
- `hw-watcher-core` and `hw-watcher-proto` forbid `unsafe`. Hardware access with `unsafe` stays isolated in
  `hw-watcher-sensors` (`bar0.rs`, `nvapi.rs`) with a `SAFETY` comment.
- No `unwrap`/`expect` outside tests, no `println!` in libraries.
- Keep Rust files under about 300 lines.

## Privacy

Do not commit telemetry recordings (`*.csv`, logs), your `config.toml`, or paths from your home
directory. `.gitignore` covers the usual cases.

## License of contributions

HW Watcher is GPL-3.0-only. By opening a pull request you agree that your contribution is released
under the same license. New source files should start with the same two-line header
(`SPDX-License-Identifier: GPL-3.0-only` and a copyright line) used by the existing files.
