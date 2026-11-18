// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Customizable widget colors, transparency, blur toggle, and built-in theme presets.

use std::collections::BTreeMap;

use hw_watcher_core::Band;
use serde::{Deserialize, Serialize};
use slint::Color;

/// Quick-pick palette swatches offered in the Control Panel color editor.
pub const PALETTE_SWATCHES: [&str; 16] = [
    "#ffffff", "#e8e8e8", "#8a8a8a", "#141418", "#56d4dd", "#4ea1ff", "#7aa2f7", "#bd93f9",
    "#ff79c6", "#ff5c5c", "#ff8c42", "#ffb347", "#f1fa8c", "#a6e3a1", "#50fa7b", "#2ec4b6",
];

/// Persistent color, transparency, and compositor blur settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Widget background hex color (`#RRGGBB`).
    pub bg_color: String,
    /// Widget background opacity percentage (`0..=100`).
    pub bg_opacity_pct: u8,
    /// Widget border hex color (`#RRGGBB`).
    pub border_color: String,
    /// Widget border opacity percentage (`0..=100`).
    pub border_opacity_pct: u8,
    /// Request X11 compositor blur behind the translucent floating widget.
    pub blur_enabled: bool,
    /// Primary normal telemetry text hex color (`#RRGGBB`).
    pub text_color: String,
    /// Muted header/secondary text hex color (`#RRGGBB`).
    pub muted_color: String,
    /// Warning threshold hex color (`#RRGGBB`).
    pub alert_color: String,
    /// Critical threshold hex color (`#RRGGBB`).
    pub critical_color: String,
    /// Taskbar badge background hex color (`#RRGGBB`).
    pub tray_bg_color: String,
    /// Taskbar badge background opacity percentage (`0..=100`).
    pub tray_bg_opacity_pct: u8,
    /// Per-item label hex colors keyed by item id (`cpu`, `gpu`, `total`, `ram`, `vram`, `board`, `cpu:0`, `gpu:0`, ...).
    pub item_label_colors: BTreeMap<String, String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg_color: "#141418".into(),
            bg_opacity_pct: 88,
            border_color: "#ffffff".into(),
            border_opacity_pct: 16,
            blur_enabled: false,
            text_color: "#e8e8e8".into(),
            muted_color: "#8a8a8a".into(),
            alert_color: "#ffb347".into(),
            critical_color: "#ff5c5c".into(),
            tray_bg_color: "#141418".into(),
            tray_bg_opacity_pct: 92,
            item_label_colors: BTreeMap::new(),
        }
    }
}

impl ThemeConfig {
    /// Clamps percentage values and normalizes invalid hex strings safely.
    pub fn sanitize(&mut self) {
        self.bg_opacity_pct = self.bg_opacity_pct.min(100);
        self.border_opacity_pct = self.border_opacity_pct.min(100);
        self.tray_bg_opacity_pct = self.tray_bg_opacity_pct.min(100);
        normalize_hex_in_place(&mut self.bg_color, "#141418");
        normalize_hex_in_place(&mut self.border_color, "#ffffff");
        normalize_hex_in_place(&mut self.text_color, "#e8e8e8");
        normalize_hex_in_place(&mut self.muted_color, "#8a8a8a");
        normalize_hex_in_place(&mut self.alert_color, "#ffb347");
        normalize_hex_in_place(&mut self.critical_color, "#ff5c5c");
        normalize_hex_in_place(&mut self.tray_bg_color, "#141418");
        self.item_label_colors
            .retain(|_, v| parse_hex_rgb(v).is_some());
        for v in self.item_label_colors.values_mut() {
            if let Some(rgb) = parse_hex_rgb(v) {
                *v = format_hex_rgb(rgb);
            }
        }
    }

    /// Resolves the configured hex color for an item key (`cpu:1` -> `cpu` -> `text_color`).
    pub fn label_color_hex(&self, key: &str) -> &str {
        if let Some(exact) = self.item_label_colors.get(key) {
            return exact;
        }
        if let Some((base, _)) = key.split_once(':')
            && let Some(parent) = self.item_label_colors.get(base)
        {
            return parent;
        }
        if key == "board" {
            return &self.muted_color;
        }
        &self.text_color
    }

    /// Returns a Slint `Color` for an item's label.
    pub fn label_color(&self, key: &str) -> Color {
        let hex = self.label_color_hex(key);
        let fallback = if key == "board" {
            [0x8a, 0x8a, 0x8a]
        } else {
            [0xe8, 0xe8, 0xe8]
        };
        color_with_opacity(hex, 100, fallback)
    }

    /// Returns the widget surface background Slint `Color` including configured transparency.
    pub fn bg_slint(&self) -> Color {
        color_with_opacity(&self.bg_color, self.bg_opacity_pct, [0x14, 0x14, 0x18])
    }

    /// Returns the widget border Slint `Color` including configured opacity.
    pub fn border_slint(&self) -> Color {
        color_with_opacity(
            &self.border_color,
            self.border_opacity_pct,
            [0xff, 0xff, 0xff],
        )
    }

    /// Returns the Slint `Color` for normal text.
    pub fn text_slint(&self) -> Color {
        color_with_opacity(&self.text_color, 100, [0xe8, 0xe8, 0xe8])
    }

    /// Returns the Slint `Color` for muted headers.
    pub fn muted_slint(&self) -> Color {
        color_with_opacity(&self.muted_color, 100, [0x8a, 0x8a, 0x8a])
    }

    /// Maps a telemetry severity [`Band`] to the customized Slint `Color`.
    pub fn band_slint(&self, band: Band) -> Color {
        match band {
            Band::Muted => self.muted_slint(),
            Band::Normal => self.text_slint(),
            Band::Alert => color_with_opacity(&self.alert_color, 100, [0xff, 0xb3, 0x47]),
            Band::Critical => color_with_opacity(&self.critical_color, 100, [0xff, 0x5c, 0x5c]),
        }
    }

    /// Maps a telemetry severity [`Band`] to RGB bytes for the 32×32 tray badge.
    pub fn band_rgb(&self, band: Band) -> [u8; 3] {
        match band {
            Band::Muted => parse_or(&self.muted_color, [0x8a, 0x8a, 0x8a]),
            Band::Normal => parse_or(&self.text_color, [0xe8, 0xe8, 0xe8]),
            Band::Alert => parse_or(&self.alert_color, [0xff, 0xb3, 0x47]),
            Band::Critical => parse_or(&self.critical_color, [0xff, 0x5c, 0x5c]),
        }
    }

    /// Returns `[A, R, G, B]` for the tray badge background.
    pub fn tray_bg_argb(&self) -> [u8; 4] {
        let rgb = parse_or(&self.tray_bg_color, [0x14, 0x14, 0x18]);
        [
            pct_to_alpha(self.tray_bg_opacity_pct),
            rgb[0],
            rgb[1],
            rgb[2],
        ]
    }

    /// Applies one of the built-in presets (`dark`, `nord`, `dracula`, `amber`, `contrast`, `glass`).
    pub fn apply_preset(&mut self, preset: &str) {
        let item_colors = std::mem::take(&mut self.item_label_colors);
        let (bg, bg_a, brd, brd_a, blur, txt, mutd, alt, crit, tbg, tbg_a) = match preset {
            "nord" => (
                "#2e3440", 90, "#88c0d0", 35, false, "#eceff4", "#81a1c1", "#ebcb8b", "#bf616a",
                "#2e3440", 95,
            ),
            "dracula" => (
                "#282a36", 90, "#bd93f9", 40, false, "#f8f8f2", "#6272a4", "#ffb86c", "#ff5555",
                "#282a36", 95,
            ),
            "amber" => (
                "#16120c", 92, "#ffb347", 30, false, "#ffd48a", "#9c7a44", "#ff9800", "#ff453a",
                "#16120c", 95,
            ),
            "contrast" => (
                "#000000", 96, "#ffffff", 60, false, "#ffffff", "#b0b0b0", "#ffd60a", "#ff453a",
                "#000000", 100,
            ),
            "glass" => (
                "#1b2028", 52, "#ffffff", 28, true, "#f2f5f9", "#b4c0d0", "#ffcc66", "#ff6b6b",
                "#1b2028", 80,
            ),
            _ => {
                *self = Self {
                    item_label_colors: item_colors,
                    ..Self::default()
                };
                return;
            }
        };
        *self = Self {
            bg_color: bg.into(),
            bg_opacity_pct: bg_a,
            border_color: brd.into(),
            border_opacity_pct: brd_a,
            blur_enabled: blur,
            text_color: txt.into(),
            muted_color: mutd.into(),
            alert_color: alt.into(),
            critical_color: crit.into(),
            tray_bg_color: tbg.into(),
            tray_bg_opacity_pct: tbg_a,
            item_label_colors: item_colors,
        };
    }
}

/// Parses `#RRGGBB` or `RRGGBB` into `[r, g, b]`.
pub fn parse_hex_rgb(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Formats `[r, g, b]` as lowercase `#rrggbb`.
pub fn format_hex_rgb(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn parse_or(raw: &str, fallback: [u8; 3]) -> [u8; 3] {
    parse_hex_rgb(raw).unwrap_or(fallback)
}

fn normalize_hex_in_place(target: &mut String, fallback: &str) {
    if let Some(rgb) = parse_hex_rgb(target) {
        *target = format_hex_rgb(rgb);
    } else {
        *target = fallback.into();
    }
}

fn pct_to_alpha(pct: u8) -> u8 {
    ((u16::from(pct.min(100)) * 255) / 100) as u8
}

/// Converts hex RGB + opacity percentage (`0..=100`) into a Slint `Color`.
pub fn color_with_opacity(hex: &str, opacity_pct: u8, fallback: [u8; 3]) -> Color {
    let [r, g, b] = parse_or(hex, fallback);
    Color::from_argb_u8(pct_to_alpha(opacity_pct), r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_resolves_hierarchical_item_colors() {
        assert_eq!(parse_hex_rgb("#4ea1ff"), Some([0x4e, 0xa1, 0xff]));
        assert_eq!(parse_hex_rgb("bad"), None);

        let mut theme = ThemeConfig::default();
        theme
            .item_label_colors
            .insert("cpu".into(), "#56d4dd".into());
        theme
            .item_label_colors
            .insert("cpu:1".into(), "#ff79c6".into());
        assert_eq!(theme.label_color_hex("cpu:0"), "#56d4dd");
        assert_eq!(theme.label_color_hex("cpu:1"), "#ff79c6");
        assert_eq!(theme.label_color_hex("gpu"), "#e8e8e8");
        assert_eq!(theme.label_color_hex("board"), "#8a8a8a");
    }
}
