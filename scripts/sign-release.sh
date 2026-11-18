#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Signs release files with the project's release key (author only; the key never leaves the author's machine).
# Usage: scripts/sign-release.sh <private-key> <file>...      -> writes <file>.sig next to each file
set -eu
KEY="${1:?usage: sign-release.sh <private-key> <file>...}"; shift
[ "$#" -gt 0 ] || { echo "no files given" >&2; exit 1; }
for f in "$@"; do
  rm -f "$f.sig"
  ssh-keygen -Y sign -f "$KEY" -n hw-watcher-release "$f"
  echo "signed: $f.sig"
done
