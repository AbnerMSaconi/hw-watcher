#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 AbnerMSaconi
# Prints the CHANGELOG.md section for one version (e.g. `scripts/release-notes.sh 0.1.0`).
# Exits non-zero when the section is missing, so a release cannot go out without notes.
set -eu
VERSION="${1:?usage: release-notes.sh <version>}"
NOTES=$(awk -v v="$VERSION" '
  $0 ~ "^## " v "([ ]|$)" { found = 1; next }
  found && /^## / { exit }
  found { print }
' "$(dirname "$0")/../CHANGELOG.md")
[ -n "$(printf '%s' "$NOTES" | tr -d '[:space:]')" ] || { echo "no CHANGELOG section for $VERSION" >&2; exit 1; }
printf '%s\n' "$NOTES"
