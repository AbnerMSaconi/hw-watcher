#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Builds a release binary that does not embed your home directory or username in panic paths.
# Use this for any binary you intend to share; CI uses the same remapping.
set -eu
cd "$(dirname "$0")/.."
RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$PWD=/src/hw-watcher --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$HOME=/home/user" \
  cargo build --release --locked "$@"
echo "built: target/release/hw-watcher"
