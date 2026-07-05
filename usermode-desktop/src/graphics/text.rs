// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Text and Font Rendering
//!
//! Provides text output functions utilizing precompiled system fonts
//! and smooth mathematical anti-aliasing scaling.

use crate::font::FONT_8X16;
use crate::graphics::core::{draw_pixel, draw_pixel_alpha};

/// Renders a single ASCII character with basic scale mapping.
pub fn draw_char(x: i32, y: i32, c: char, r: u8, g: u8, b: u8, scale: i32) {
    let idx = c as usize;
    if idx >= 128 {
        return;
    }
    let glyph = FONT_8X16[idx];
    for row in 0..16 {
        let row_data = glyph[row];
        for col in 0..8 {
            if (row_data & (0x80 >> col)) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        draw_pixel(x + (col as i32) * scale + sx, y + (row as i32) * scale + sy, r, g, b);
                    }
                }
            }
        }
    }
}

/// Renders a string of text.
pub fn draw_string(x: i32, y: i32, s: &str, r: u8, g: u8, b: u8, scale: i32) {
    let mut cur_x = x;
    for c in s.chars() {
        if c == '\n' {
            continue;
        }
        draw_char(cur_x, y, c, r, g, b, scale);
        cur_x += 8 * scale + 1;
    }
}

/// Renders a scaled character with sub-pixel mathematical anti-aliasing.
pub fn draw_char_smooth(x: i32, y: i32, c: char, r: u8, g: u8, b: u8, size_w: i32, size_h: i32) {
    let idx = c as usize;
    if idx >= 128 { return; }
    let glyph = FONT_8X16[idx];

    let scale_x = (8 * 256) / size_w;
    let scale_y = (16 * 256) / size_h;

    for dy in 0..size_h {
        let sy_top = (((dy * 4 + 1) * scale_y) / 4) >> 8;
        let sy_bottom = (((dy * 4 + 3) * scale_y) / 4) >> 8;
        
        for dx in 0..size_w {
            let sx_left = (((dx * 4 + 1) * scale_x) / 4) >> 8;
            let sx_right = (((dx * 4 + 3) * scale_x) / 4) >> 8;

            let mut active_subpixels = 0;

            if sx_left >= 0 && sx_left < 8 && sy_top >= 0 && sy_top < 16 {
                if (glyph[sy_top as usize] & (0x80 >> sx_left)) != 0 { active_subpixels += 1; }
            }

            if sx_right >= 0 && sx_right < 8 && sy_top >= 0 && sy_top < 16 {
                if (glyph[sy_top as usize] & (0x80 >> sx_right)) != 0 { active_subpixels += 1; }
            }

            if sx_left >= 0 && sx_left < 8 && sy_bottom >= 0 && sy_bottom < 16 {
                if (glyph[sy_bottom as usize] & (0x80 >> sx_left)) != 0 { active_subpixels += 1; }
            }

            if sx_right >= 0 && sx_right < 8 && sy_bottom >= 0 && sy_bottom < 16 {
                if (glyph[sy_bottom as usize] & (0x80 >> sx_right)) != 0 { active_subpixels += 1; }
            }

            if active_subpixels > 0 {
                let alpha = match active_subpixels {
                    1 => 64,
                    2 => 128,
                    3 => 192,
                    _ => 255,
                };
                draw_pixel_alpha(x + dx, y + dy, r, g, b, alpha);
            }
        }
    }
}

/// Renders a string of anti-aliased text.
pub fn draw_string_smooth(x: i32, y: i32, s: &str, r: u8, g: u8, b: u8, char_w: i32, char_h: i32) {
    let mut cur_x = x;
    for c in s.chars() {
        if c == '\n' {
            continue;
        }
        draw_char_smooth(cur_x, y, c, r, g, b, char_w, char_h);
        cur_x += char_w + 1;
    }
}
