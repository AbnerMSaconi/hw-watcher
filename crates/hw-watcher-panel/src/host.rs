// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Computer name shown in the widget header.

use std::fs;

/// Longest name shown; longer host names are cut so they never crowd the header.
const MAX_CHARS: usize = 24;

/// Name of this computer: `HW_WATCHER_NAME` when set (useful for demos and screenshots),
/// otherwise the system host name. Empty when neither is available.
pub fn computer_name() -> String {
    let raw = std::env::var("HW_WATCHER_NAME")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| fs::read_to_string("/proc/sys/kernel/hostname").ok())
        .or_else(|| fs::read_to_string("/etc/hostname").ok())
        .unwrap_or_default();
    clip(raw.trim())
}

fn clip(name: &str) -> String {
    if name.chars().count() <= MAX_CHARS {
        return name.to_owned();
    }
    let head: String = name.chars().take(MAX_CHARS - 1).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::clip;

    #[test]
    fn keeps_short_names_and_clips_long_ones() {
        assert_eq!(clip("my-pc"), "my-pc");
        let long = "a".repeat(40);
        let out = clip(&long);
        assert_eq!(out.chars().count(), 24);
        assert!(out.ends_with('…'));
    }
}
