// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

use crate::atlas_font::{draw_text_atlas, AtlasSize, AtlasWeight};
use crate::graphics::{draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha};

/// Renders the visual content of the System Settings window.
///
/// This application window allows users to toggle desktop rendering parameters,
/// specifically the translucent window drop shadows. If shadows are disabled,
/// the desktop bypasses shadow layer rendering, leading to a significant
/// performance boost.
///
/// # Arguments
///
/// * `ax` - The animated starting X-coordinate of the window body.
/// * `ay` - The animated starting Y-coordinate of the window body.
/// * `w` - The active width of the window.
/// * `h` - The active height of the window.
pub fn draw_settings_window(ax: i32, ay: i32, w: usize, _h: usize) {
    let shadows_on = crate::state::SHADOWS_ENABLED.load(core::sync::atomic::Ordering::Relaxed);
    
    // Header/Description
    draw_text_atlas(ax + 24, ay + 50, "Desktop Performance", 230, 235, 245, AtlasSize::Small, AtlasWeight::SemiBold);
    draw_text_atlas(ax + 24, ay + 72, "Toggle options to adjust rendering speed.", 150, 155, 170, AtlasSize::Small, AtlasWeight::Regular);
    
    // Option Card for Window Shadows
    let toggle_y = ay + 110;
    // Base container card
    draw_rounded_rect_alpha(ax + 24, toggle_y, w as i32 - 48, 56, 8, 32, 34, 42, 235);
    draw_rounded_rect_outline_alpha(ax + 24, toggle_y, w as i32 - 48, 56, 8, 60, 64, 80, 1, 100);
    
    // Label for the setting
    draw_text_atlas(ax + 40, toggle_y + 18, "Enable Window Shadows", 220, 225, 235, AtlasSize::Small, AtlasWeight::Regular);
    
    // Switch bounds and coordinates on the right side of the card
    let switch_x = ax + w as i32 - 96;
    let switch_y = toggle_y + 16;
    let switch_w = 48;
    let switch_h = 24;
    
    if shadows_on {
        // Switch Active (Blue pill)
        draw_rounded_rect_alpha(switch_x, switch_y, switch_w, switch_h, 12, 61, 174, 233, 255);
        // Switch Knob (Right-aligned white dot)
        draw_rounded_rect_alpha(switch_x + 26, switch_y + 2, 20, 20, 10, 255, 255, 255, 255);
    } else {
        // Switch Inactive (Dark gray pill)
        draw_rounded_rect_alpha(switch_x, switch_y, switch_w, switch_h, 12, 50, 52, 64, 255);
        // Switch Knob (Left-aligned white dot)
        draw_rounded_rect_alpha(switch_x + 2, switch_y + 2, 20, 20, 10, 255, 255, 255, 255);
    }
}
