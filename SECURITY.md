# Security and privacy

HW Watcher reads hardware sensors and shows them. This page says plainly what it can access, what
it does not do, and what has **not** been verified.

## Privacy

- **No network access.** The code and its dependencies contain no HTTP, TLS or socket (TCP/UDP)
  support. The panel talks only to the local D-Bus, and the system service is restricted to
  `AF_UNIX`. There is no telemetry, update check or "phone home".
- **Nothing is recorded.** Readings live in memory only. The only file written is your settings in
  `~/.config/hw-watcher/config.toml` (window position and preferences).
- **Output identifies the program, not you.** `hw-watcher about` and the embedded provenance record
  name the author and project. `scripts/hw-report.sh` prints no hostname, serial number, MAC
  address or user name.

## What the optional daemon can do

Running the panel as a normal user needs no privileges. Only if you install the system service
(`sudo hw-watcher service install --system`) does a daemon run as root with
`CAP_DAC_OVERRIDE`, `CAP_DAC_READ_SEARCH` and `CAP_SYS_RAWIO`, to read RAPL counters and the GPU's
PCI BAR0 registers (read-only memory mapping). Its limits:

- sandboxed by systemd (`ProtectSystem=strict`, `ProtectHome=yes`, `NoNewPrivileges=yes`,
  `RestrictAddressFamilies=AF_UNIX`, capability bounding set as above);
- the D-Bus policy lets every user call only `Sample` and `Capabilities` (read the latest
  readings); nothing can write, configure or execute through it;
- BAR0 access is limited to a verified GPU (GA102) and fixed, read-only register offsets.

If you do not need CPU package power or VRAM/hotspot temperature, do not install the daemon.

## What has not been verified

- **No independent security audit** has been done. The code is reviewed only by its author, with
  automated tests and `clippy`.
- **`unsafe` code** exists only in `hw-watcher-sensors` (`bar0.rs`, memory-mapped register reads;
  `nvapi.rs`, loading NVIDIA's library). The other crates deny or forbid it. Reviews of these two
  files are welcome.
- **Dependencies** (several hundred crates, mostly from the Slint and async ecosystems) are not
  audited line by line by this project. `Cargo.lock` is committed, release builds use `--locked`,
  and CI runs `cargo deny` (`deny.toml`) on every push: it fails on any known vulnerability in the
  RustSec database, on licenses incompatible with GPL-3.0 and on unknown registries. When last run
  it reported no known vulnerabilities, only six transitive crates flagged as unmaintained
  (`zvariant`, `zvariant_derive`, `zbus_names`, `ttf-parser`, `pin-utils`, `chrono`) with no fixed
  release to move to.
- AMD, Intel and generic board sensors have never run on real hardware (see
  `docs/COMPATIBILITY.md`). They only read sysfs files, but a wrong mapping could show wrong values.

## Getting a trustworthy binary

- Download from the Releases page and verify it as described in `docs/PROVENANCE.md`
  (`sha256sum -c` and `gh attestation verify`).
- If you share a binary you built yourself, use `scripts/build-release.sh`. A plain
  `cargo build --release` embeds your home directory and user name in panic messages.

## Reporting a vulnerability

Please report privately through the repository's **Report a vulnerability** button (GitHub security
advisories) instead of a public issue. Include the version (`hw-watcher --version`) and, if it is
about the daemon, your distribution and how the service was installed.
