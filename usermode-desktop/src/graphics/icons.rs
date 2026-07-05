// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Interface Icons
//!
//! Provides vector drawing routines for application and folders icons.

use crate::graphics::core::{draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha, draw_rect_alpha, draw_line_thick};

/// Legacy desktop sidebar icons drawing routine.
pub fn draw_icon(id: u8, x: i32, y: i32) {
    if id == 0 {
        // Files — folder icon
        draw_rounded_rect_alpha(x + 2,  y + 2,  22,  8, 4,  55, 155, 230, 255);
        draw_rounded_rect_alpha(x,      y + 8,  44, 34, 7,  48, 142, 218, 255);
        draw_rounded_rect_alpha(x + 3,  y + 9,  38,  8, 3, 130, 210, 255,  45);
        draw_rounded_rect_alpha(x + 2, y + 32,  40,  6, 3,  25,  90, 160,  60);
    } else if id == 1 {
        // Console — dark window card
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 42, 8,  22,  24,  36, 255);
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 14, 8,  38,  42,  60, 255);
        draw_rect_alpha(        x + 2, y + 8, 44,  8, 38,  42,  60, 255);
        draw_rounded_rect_alpha(x +  8, y + 5, 6, 6, 3, 232,  74,  58, 255);
        draw_rounded_rect_alpha(x + 18, y + 5, 6, 6, 3, 242, 186,  26, 255);
        draw_rounded_rect_alpha(x + 28, y + 5, 6, 6, 3,  56, 200,  90, 255);
        draw_rect_alpha(x +  8, y + 22, 20, 3,  61, 174, 233, 210);
        draw_rect_alpha(x +  8, y + 29, 28, 3, 200, 205, 215,  90);
        draw_rect_alpha(x +  8, y + 36, 16, 3, 200, 205, 215,  60);
    } else if id == 2 {
        // Metrics — chart card
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 42, 8,  26,  30,  46, 255);
        draw_rounded_rect_alpha(x +  6, y + 26, 8, 14, 3,  61, 174, 233, 255);
        draw_rounded_rect_alpha(x + 18, y + 16, 8, 24, 3,  90, 200, 150, 255);
        draw_rounded_rect_alpha(x + 30, y + 21, 8, 19, 3, 180, 100, 230, 255);
        draw_rect_alpha(x + 4, y + 38, 38, 1, 255, 255, 255, 30);
    }
}

/// Renders a tiny folder icon (used in file manager).
pub fn draw_tiny_folder_icon(x: i32, y: i32) {
    draw_rounded_rect_alpha(x + 1, y,      8,  4, 2,  61, 174, 233, 255);
    draw_rounded_rect_alpha(x,     y + 3, 16, 10, 3,  47, 140, 215, 255);
    draw_rounded_rect_alpha(x + 1, y + 4, 14,  3, 1, 120, 200, 255,  45);
}

/// Renders a tiny document document icon (used in file manager).
pub fn draw_tiny_file_icon(x: i32, y: i32) {
    draw_rounded_rect_alpha(x + 2, y,      12, 14, 2, 215, 220, 232, 255);
    draw_rounded_rect_alpha(x + 9, y,       5,  5, 1, 175, 180, 198, 255);
    draw_rect_alpha(x + 4, y + 5,  7, 1, 130, 135, 155, 210);
    draw_rect_alpha(x + 4, y + 8,  7, 1, 130, 135, 155, 180);
    draw_rect_alpha(x + 4, y + 11, 5, 1, 130, 135, 155, 150);
}

/// Renders modern vector launchpad icon for Dock.
pub fn draw_vector_launchpad_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 50, 50, 60, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 100, 100, 120, 1, 100);
    let dot_w = (size as f32 * 0.12) as i32;
    let spacing = (size as f32 * 0.12) as i32;
    let start_offset = (size as f32 * 0.22) as i32;
    for row in 0..3 {
        for col in 0..3 {
            let dx = start_offset + col * (dot_w + spacing);
            let dy = start_offset + row * (dot_w + spacing);
            draw_rounded_rect_alpha(x + dx, y + dy, dot_w.max(2), dot_w.max(2), (dot_w/2).max(1), 255, 255, 255, 230);
        }
    }
}

/// Renders modern vector folder icon for Dock.
pub fn draw_vector_folder_icon(x: i32, y: i32, size: i32) {
    let r_tab = (size as f32 * 0.08) as i32;
    let r_body = (size as f32 * 0.15) as i32;
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.05) as i32,
        y + (size as f32 * 0.05) as i32,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.2) as i32,
        r_tab.max(2),
        55, 155, 230, 255
    );
    draw_rounded_rect_alpha(
        x,
        y + (size as f32 * 0.18) as i32,
        size,
        (size as f32 * 0.82) as i32,
        r_body.max(3),
        48, 142, 218, 255
    );
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.08) as i32,
        y + (size as f32 * 0.22) as i32,
        (size as f32 * 0.84) as i32,
        (size as f32 * 0.15) as i32,
        r_tab.max(1),
        130, 210, 255, 45
    );
}

/// Renders modern vector terminal icon for Dock.
pub fn draw_vector_terminal_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 20, 22, 34, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 60, 65, 80, 1, 100);
    let dot_size = (size as f32 * 0.08) as i32;
    let dot_y = y + (size as f32 * 0.15) as i32;
    let r_dot = (dot_size as f32 * 0.5) as i32;
    draw_rounded_rect_alpha(x + (size as f32 * 0.15) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 230, 72, 58, 255);
    draw_rounded_rect_alpha(x + (size as f32 * 0.3) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 240, 185, 25, 255);
    draw_rounded_rect_alpha(x + (size as f32 * 0.45) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 39, 201, 63, 255);
    let px = x + (size as f32 * 0.25) as i32;
    let py = y + (size as f32 * 0.5) as i32;
    let pw = (size as f32 * 0.15) as i32;
    draw_line_thick(px, py, px + pw, py + pw / 2, 230, 235, 250);
    draw_line_thick(px + pw, py + pw / 2, px, py + pw, 230, 235, 250);
}

/// Renders modern vector system metrics icon for Dock.
pub fn draw_vector_metrics_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 32, 36, 54, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 80, 85, 105, 1, 80);
    let bar_w = (size as f32 * 0.18) as i32;
    let bar_r = (bar_w as f32 * 0.3) as i32;
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.18) as i32,
        y + (size as f32 * 0.5) as i32,
        bar_w.max(2),
        (size as f32 * 0.35) as i32,
        bar_r.max(1),
        61, 174, 233, 255
    );
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.41) as i32,
        y + (size as f32 * 0.25) as i32,
        bar_w.max(2),
        (size as f32 * 0.6) as i32,
        bar_r.max(1),
        90, 200, 150, 255
    );
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.64) as i32,
        y + (size as f32 * 0.4) as i32,
        bar_w.max(2),
        (size as f32 * 0.45) as i32,
        bar_r.max(1),
        180, 100, 230, 255
    );
}

/// Renders modern settings/performance toggle vector icon.
pub fn draw_vector_settings_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 44, 48, 64, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 110, 115, 140, 1, 100);
    let track_w = (size as f32 * 0.6) as i32;
    let track_x = x + (size as f32 * 0.2) as i32;
    let start_y = y + (size as f32 * 0.25) as i32;
    let step_y = (size as f32 * 0.25) as i32;
    for i in 0..3 {
        let ty = start_y + i * step_y;
        draw_rounded_rect_alpha(track_x, ty, track_w, 2, 1, 80, 85, 105, 180);
        let knob_offset = match i {
            0 => (size as f32 * 0.3) as i32,
            1 => (size as f32 * 0.5) as i32,
            2 => (size as f32 * 0.2) as i32,
            _ => 0,
        };
        let knob_x = track_x + knob_offset;
        draw_rounded_rect_alpha(knob_x - 4, ty - 3, 8, 8, 4, 61, 174, 233, 255);
    }
}

/// Renders a modern radioactive hazard symbol vector icon for the Radiation Simulator.
pub fn draw_vector_radiation_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    // Dark violet-gray container
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 32, 28, 44, 255);
    // Orange glowing border
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 235, 140, 40, 1, 100);
    
    let cx = x + size / 2;
    let cy = y + size / 2;
    
    // Central core dot
    draw_rounded_rect_alpha(cx - 3, cy - 3, 6, 6, 3, 235, 140, 40, 255);
    
    // Trefoil blades
    // Top blade
    draw_rect_alpha(cx - 2, cy - 13, 4, 7, 235, 140, 40, 255);
    // Bottom-left blade
    draw_rect_alpha(cx - 10, cy + 5, 7, 4, 235, 140, 40, 255);
    // Bottom-right blade
    draw_rect_alpha(cx + 3, cy + 5, 7, 4, 235, 140, 40, 255);
}
