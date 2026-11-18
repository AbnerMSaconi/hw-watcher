#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Installs hw-watcher for the current user (no root): binary, launcher entry, autostart.
# Usage: ./scripts/install.sh [path/to/hw-watcher]
set -eu
BIN="${1:-./target/release/hw-watcher}"
[ -x "$BIN" ] || { echo "binary not found: $BIN (run 'cargo build --release' first)" >&2; exit 1; }
DEST="${XDG_BIN_HOME:-$HOME/.local/bin}"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
AUTO="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
mkdir -p "$DEST" "$APPS" "$AUTO"
install -m 755 "$BIN" "$DEST/hw-watcher"
ln -sf hw-watcher "$DEST/hww"   # short alias
cat > "$APPS/hw-watcher.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=HW Watcher
Comment=Floating CPU, GPU, RAM and VRAM telemetry panel
Exec=$DEST/hw-watcher panel
Terminal=false
Categories=System;Monitor;
DESK
sed 's|panel$|panel --tray|' "$APPS/hw-watcher.desktop" > "$AUTO/hw-watcher.desktop"
echo "Installed to $DEST/hw-watcher (alias: hww, autostart enabled; remove $AUTO/hw-watcher.desktop to disable)."
