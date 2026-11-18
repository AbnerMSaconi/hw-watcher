#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Verifies a release file against the project's public release key (docs/keys/release-signers).
# Usage: scripts/verify-release.sh <file>     (expects <file>.sig next to it)
set -eu
FILE="${1:?usage: verify-release.sh <file>}"
SIGNERS="${SIGNERS:-$(dirname "$0")/../docs/keys/release-signers}"
[ -f "$FILE.sig" ] || { echo "missing signature: $FILE.sig" >&2; exit 1; }
ssh-keygen -Y verify -f "$SIGNERS" -I release@hw-watcher -n hw-watcher-release -s "$FILE.sig" < "$FILE"
