// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! 32x32 ARGB32 bitmap badge icon renderer for SNI system tray slots.

use hw_watcher_core::TrayRendering;
use ksni::Icon;

use crate::theme::ThemeConfig;

const ICON_SIZE: usize = 32;

/// 3x5 bitmap glyphs (bit 2 = left column).
fn glyph(ch: char) -> Option<[u8; 5]> {
    Some(match ch {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 1, 1],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        '.' => [0, 0, 0, 0, 2],
        '%' => [5, 1, 2, 4, 5],
        'G' => [7, 4, 5, 5, 7],
        'E' => [7, 4, 6, 4, 7],
        '—' | '-' => [0, 0, 7, 0, 0],
        _ => return None,
    })
}

/// Renders a 32×32 ARGB32 rounded badge icon using `theme`.
pub fn render_themed_badge(rendering: &TrayRendering, theme: &ThemeConfig) -> Icon {
    let mut px = vec![[0u8; 4]; ICON_SIZE * ICON_SIZE];
    fill_rounded_bg(&mut px, theme.tray_bg_argb());

    let chars: Vec<char> = rendering.text.chars().take(4).collect();
    let (scale, step) = if chars.len() <= 2 { (3, 4) } else { (2, 3) };
    let total_w = (chars.len() * step * scale).saturating_sub(scale);
    let x0 = ICON_SIZE.saturating_sub(total_w) / 2;
    let y0 = ICON_SIZE.saturating_sub(5 * scale) / 2;
    let rgb = theme.band_rgb(rendering.band);

    for (idx, ch) in chars.into_iter().enumerate() {
        let Some(rows) = glyph(ch) else { continue };
        draw_glyph(&mut px, rows, x0 + idx * step * scale, y0, scale, rgb);
    }

    Icon {
        width: ICON_SIZE as i32,
        height: ICON_SIZE as i32,
        data: px.into_iter().flatten().collect(),
    }
}

fn fill_rounded_bg(px: &mut [[u8; 4]], argb: [u8; 4]) {
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let (cx, cy) = (x.min(ICON_SIZE - 1 - x), y.min(ICON_SIZE - 1 - y));
            let outside_corner = cx < 5 && cy < 5 && (5 - cx).pow(2) + (5 - cy).pow(2) > 26;
            if !outside_corner {
                px[y * ICON_SIZE + x] = argb;
            }
        }
    }
}

fn draw_glyph(px: &mut [[u8; 4]], rows: [u8; 5], gx: usize, gy: usize, scale: usize, rgb: [u8; 3]) {
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (4 >> col) != 0 {
                fill_block(px, gx + col * scale, gy + row * scale, scale, rgb);
            }
        }
    }
}

fn fill_block(px: &mut [[u8; 4]], x0: usize, y0: usize, scale: usize, rgb: [u8; 3]) {
    for dy in 0..scale {
        for dx in 0..scale {
            let (x, y) = (x0 + dx, y0 + dy);
            if x < ICON_SIZE && y < ICON_SIZE {
                px[y * ICON_SIZE + x] = [255, rgb[0], rgb[1], rgb[2]];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hw_watcher_core::Band;

    #[test]
    fn badge_icon_has_expected_argb_size_for_all_lengths() {
        for text in ["—", "E", "42", "165", "1.9G", "100%"] {
            let icon = render_themed_badge(
                &TrayRendering {
                    text: text.into(),
                    band: Band::Normal,
                },
                &ThemeConfig::default(),
            );
            assert_eq!(icon.data.len(), ICON_SIZE * ICON_SIZE * 4);
        }
    }
}
