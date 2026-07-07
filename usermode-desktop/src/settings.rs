// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Settings Application Interface
//!
//! Provides the visual rendering of the sidebar-style Settings application window,
//! featuring sidebar navigation on the left and dynamic content panes on the right.

use crate::atlas_font::{draw_text_atlas, AtlasSize, AtlasWeight};
use crate::graphics::{draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha, draw_rect_alpha};
use crate::syscalls::SharedSystemInfo;
use crate::utils::StrbufWriter;
use core::sync::atomic::Ordering;

/// Renders the visual content of the sidebar-style System Settings window.
///
/// Spans a sidebar panel on the left for category selection (Appearance, System, About)
/// and displays the selected options on the right side.
///
/// # Arguments
///
/// * `ax` - The animated starting X-coordinate of the window body.
/// * `ay` - The animated starting Y-coordinate of the window body.
/// * `w` - The active width of the window.
/// * `h` - The active height of the window.
/// * `cursor_x` - The current X-coordinate of the mouse cursor.
/// * `cursor_y` - The current Y-coordinate of the mouse cursor.
/// * `shared_info` - Pointer to the shared microkernel information structure.
pub fn draw_settings_window(
    ax: i32,
    ay: i32,
    w: usize,
    h: usize,
    cursor_x: i32,
    cursor_y: i32,
    shared_info: *const SharedSystemInfo,
) {
    let active_tab = crate::state::ACTIVE_SETTINGS_TAB.load(Ordering::Relaxed);

    // ────────────────────────────────────────────────────────
    // 1. Sidebar Panel (Left Side, Width 130px)
    // ────────────────────────────────────────────────────────
    let tabs = ["Appearance", "System", "About"];
    for (i, tab_title) in tabs.iter().enumerate() {
        let tab_y = ay + 50 + (i as i32) * 32;
        let is_active = active_tab == i as u32;
        let is_hovered = cursor_x >= ax + 10 && cursor_x < ax + 120 &&
                         cursor_y >= tab_y && cursor_y < tab_y + 26;

        if is_active {
            // Muted blue highlight for the active sidebar selection
            draw_rounded_rect_alpha(ax + 10, tab_y, 110, 26, 6, 61, 174, 233, 255);
            draw_text_atlas(ax + 20, tab_y + 6, tab_title, 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);
        } else if is_hovered {
            // Subtle gray highlight for hovered items
            draw_rounded_rect_alpha(ax + 10, tab_y, 110, 26, 6, 255, 255, 255, 20);
            draw_text_atlas(ax + 20, tab_y + 6, tab_title, 230, 235, 245, AtlasSize::Small, AtlasWeight::Regular);
        } else {
            draw_text_atlas(ax + 20, tab_y + 6, tab_title, 170, 175, 190, AtlasSize::Small, AtlasWeight::Regular);
        }
    }

    // Vertical Divider separating sidebar from content pane
    draw_rect_alpha(ax + 130, ay + 34, 1, h as i32 - 34, 60, 65, 80, 100);

    // ────────────────────────────────────────────────────────
    // 2. Right Pane (Active Category Contents)
    // ────────────────────────────────────────────────────────
    let content_x = ax + 150;
    let card_w = w as i32 - 174;

    match active_tab {
        0 => {
            // Appearance Tab
            draw_text_atlas(content_x, ay + 50, "Appearance Settings", 230, 235, 245, AtlasSize::Small, AtlasWeight::SemiBold);
            draw_text_atlas(content_x, ay + 72, "Adjust windows aesthetics and shadows.", 150, 155, 170, AtlasSize::Small, AtlasWeight::Regular);

            let toggle_y = ay + 110;
            // Option container card
            draw_rounded_rect_alpha(content_x, toggle_y, card_w, 56, 8, 32, 34, 42, 235);
            draw_rounded_rect_outline_alpha(content_x, toggle_y, card_w, 56, 8, 60, 64, 80, 1, 100);

            draw_text_atlas(content_x + 16, toggle_y + 18, "Enable Window Shadows", 220, 225, 235, AtlasSize::Small, AtlasWeight::Regular);

            let switch_x = ax + w as i32 - 84;
            let switch_y = toggle_y + 16;
            let shadows_on = crate::state::SHADOWS_ENABLED.load(Ordering::Relaxed);

            if shadows_on {
                // Switch Active (Blue pill)
                draw_rounded_rect_alpha(switch_x, switch_y, 48, 24, 12, 61, 174, 233, 255);
                // Switch Knob (Right-aligned)
                draw_rounded_rect_alpha(switch_x + 26, switch_y + 2, 20, 20, 10, 255, 255, 255, 255);
            } else {
                // Switch Inactive (Dark gray pill)
                draw_rounded_rect_alpha(switch_x, switch_y, 48, 24, 12, 50, 52, 64, 255);
                // Switch Knob (Left-aligned)
                draw_rounded_rect_alpha(switch_x + 2, switch_y + 2, 20, 20, 10, 255, 255, 255, 255);
            }
        }
        1 => {
            // System Tab
            draw_text_atlas(content_x, ay + 50, "System Performance", 230, 235, 245, AtlasSize::Small, AtlasWeight::SemiBold);

            let card_y = ay + 90;
            draw_rounded_rect_alpha(content_x, card_y, card_w, 120, 8, 32, 34, 42, 235);
            draw_rounded_rect_outline_alpha(content_x, card_y, card_w, 120, 8, 60, 64, 80, 1, 100);

            let mut stat_buf = [0u8; 64];

            // 1. ECC corrections
            let ecc = unsafe { (*shared_info).ecc_corrections.load(Ordering::Relaxed) };
            let mut w_ecc = StrbufWriter::new(&mut stat_buf);
            let _ = core::fmt::write(&mut w_ecc, format_args!("ECC Corrections: {}", ecc));
            draw_text_atlas(content_x + 16, card_y + 16, w_ecc.as_str(), 200, 205, 220, AtlasSize::Small, AtlasWeight::Regular);

            // 2. Quarantined pages
            let quar = unsafe { (*shared_info).pages_quarantined.load(Ordering::Relaxed) };
            let mut w_quar = StrbufWriter::new(&mut stat_buf);
            let _ = core::fmt::write(&mut w_quar, format_args!("Pages Quarantined: {}", quar));
            draw_text_atlas(content_x + 16, card_y + 48, w_quar.as_str(), 200, 205, 220, AtlasSize::Small, AtlasWeight::Regular);

            // 3. RAM usage
            let heap_used = unsafe { (*shared_info).heap_used.load(Ordering::Relaxed) };
            let mut w_ram = StrbufWriter::new(&mut stat_buf);
            let _ = core::fmt::write(&mut w_ram, format_args!("Memory Used: {} KB", heap_used / 1024));
            draw_text_atlas(content_x + 16, card_y + 80, w_ram.as_str(), 200, 205, 220, AtlasSize::Small, AtlasWeight::Regular);
        }
        2 => {
            // About Tab
            draw_text_atlas(content_x, ay + 50, "About AE Rustanium", 230, 235, 245, AtlasSize::Small, AtlasWeight::SemiBold);

            let card_y = ay + 90;
            draw_rounded_rect_alpha(content_x, card_y, card_w, 120, 8, 32, 34, 42, 235);
            draw_rounded_rect_outline_alpha(content_x, card_y, card_w, 120, 8, 60, 64, 80, 1, 100);

            draw_text_atlas(content_x + 16, card_y + 16, "OS: AE Rustanium v0.1.0", 220, 225, 235, AtlasSize::Small, AtlasWeight::SemiBold);
            draw_text_atlas(content_x + 16, card_y + 44, "Arch: x86_64 UEFI Dual-Boot", 180, 185, 200, AtlasSize::Small, AtlasWeight::Regular);
            draw_text_atlas(content_x + 16, card_y + 68, "Safety: 100% Zero-Unsafe Core", 180, 185, 200, AtlasSize::Small, AtlasWeight::Regular);
            draw_text_atlas(content_x + 16, card_y + 92, "Resilience: Fault-Tolerant SECDED", 180, 185, 200, AtlasSize::Small, AtlasWeight::Regular);
        }
        _ => {}
    }
}
