#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Prints non-identifying hardware/session info useful for sensor-support bug reports.
# It never prints hostnames, serial numbers, MAC addresses or user names.
set -u
echo "## Session"
if command -v hw-watcher >/dev/null 2>&1; then echo "version: $(hw-watcher --version)"; elif [ -x ./target/release/hw-watcher ]; then echo "version: $(./target/release/hw-watcher --version)"; fi
echo "kernel: $(uname -r)"
echo "arch: $(uname -m)"
echo "session: ${XDG_SESSION_TYPE:-unknown} / desktop: ${XDG_CURRENT_DESKTOP:-unknown}"
[ -r /etc/os-release ] && . /etc/os-release && echo "distro: ${PRETTY_NAME:-unknown}"
echo
echo "## hwmon chips"
for d in /sys/class/hwmon/hwmon*; do
  [ -r "$d/name" ] && echo "$(basename "$d"): $(cat "$d/name")"
done
echo
echo "## DRM GPUs (driver)"
for c in /sys/class/drm/card[0-9]*; do
  case "$c" in *-*) continue ;; esac
  [ -r "$c/device/uevent" ] && echo "$(basename "$c"): $(sed -n 's/^DRIVER=//p' "$c/device/uevent")"
done
echo
echo "## RAPL"
ls /sys/class/powercap 2>/dev/null | grep -i rapl || echo "none"
echo
echo "## hw-watcher sample"
if command -v hw-watcher >/dev/null 2>&1; then
  hw-watcher sample --json
elif [ -x ./target/release/hw-watcher ]; then
  ./target/release/hw-watcher sample --json
else
  echo "hw-watcher not found in PATH or ./target/release"
fi
