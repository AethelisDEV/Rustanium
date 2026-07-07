// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

use crate::utils::StrbufWriter;
use crate::atlas_font::{draw_text_atlas, AtlasSize, AtlasWeight};
use crate::graphics::{
    draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha,
    draw_start_menu, draw_rect_alpha
};
use crate::graphics::{
    draw_vector_launchpad_icon, draw_vector_metrics_icon, draw_vector_folder_icon,
    draw_vector_terminal_icon, draw_vector_radiation_icon
};
use crate::state::{START_MENU_OPEN, START_MENU_ANIMATING, START_MENU_ANIM_PROGRESS};
use crate::window::WINDOWS;
use crate::syscalls::SharedSystemInfo;
use core::sync::atomic::Ordering;

/// Calculate the dynamic, magnified layout coordinates of the Dock.
/// Returns: (start_x, total_w, sizes, xs)
pub fn get_dock_layout(sw: i32, sh: i32, cursor_x: i32, cursor_y: i32) -> (f32, f32, [f32; 5], [f32; 5]) {
    let base_size = 36.0f32;
    let max_size = 52.0f32;
    let spacing = 12.0f32;
    let range = 100.0f32;
    
    let unscaled_item_w = base_size + spacing;
    let unscaled_total_w = 5.0 * unscaled_item_w + spacing;
    let unscaled_start_x = (sw as f32 - unscaled_total_w) / 2.0;
    
    // Magnification only triggers if cursor is near the bottom area
    let near_dock = cursor_y >= (sh - 80);
    
    let mut sizes = [0.0f32; 5];
    for i in 0..5 {
        let unscaled_cx = unscaled_start_x + spacing + (i as f32) * unscaled_item_w + base_size / 2.0;
        let dist_x = (cursor_x as f32 - unscaled_cx).abs();
        if near_dock && dist_x < range {
            let t = 1.0 - (dist_x / range);
            // Smooth ease curve
            let ease = t * t * (3.0 - 2.0 * t);
            sizes[i] = base_size + (max_size - base_size) * ease;
        } else {
            sizes[i] = base_size;
        }
    }
    
    let mut total_w = spacing;
    for i in 0..5 {
        total_w += sizes[i] + spacing;
    }
    let start_x = (sw as f32 - total_w) / 2.0;
    
    let mut xs = [0.0f32; 5];
    xs[0] = start_x + spacing;
    for i in 1..5 {
        xs[i] = xs[i - 1] + sizes[i - 1] + spacing;
    }
    
    (start_x, total_w, sizes, xs)
}

pub fn draw_taskbar(
    sw: i32,
    sh: i32,
    cursor_x: i32,
    cursor_y: i32,
    _ticks: u64,
    shared_info: *const SharedSystemInfo,
) {
    // ────────────────────────────────────────────────────────
    // 1. macOS Top Menu Bar
    // ────────────────────────────────────────────────────────
    // Translucent dark menu bar
    draw_rect_alpha(0, 0, sw, 24, 20, 20, 24, 210);
    // Subtle bottom divider
    draw_rect_alpha(0, 24, sw, 1, 55, 60, 80, 80);
    
    // Left Branding
    draw_text_atlas(12, 5, "AE Rustanium", 225, 230, 245, AtlasSize::Small, AtlasWeight::SemiBold);
    
    // Stats on the right
    let mut stats_buf = [0u8; 64];
    let mut stats_writer = StrbufWriter::new(&mut stats_buf);
    let cpu_load = unsafe { (*shared_info).cpu_usage.load(Ordering::Relaxed) };
    let heap_used = unsafe { (*shared_info).heap_used.load(Ordering::Relaxed) };
    let _ = core::fmt::write(&mut stats_writer, format_args!("CPU {}.{:02}%   RAM {} MB used", cpu_load / 100, cpu_load % 100, heap_used / 1024 / 1024));
    draw_text_atlas(sw - 230, 5, stats_writer.as_str(), 190, 200, 220, AtlasSize::Small, AtlasWeight::Regular);

    // ────────────────────────────────────────────────────────
    // 2. macOS Floating Dock
    // ────────────────────────────────────────────────────────
    let (start_x, total_w, sizes, xs) = get_dock_layout(sw, sh, cursor_x, cursor_y);
    let dock_h = 56;
    let dock_y = sh - dock_h - 6; // sh - 62
    
    // Draw Frosted Glass background
    draw_rounded_rect_alpha(start_x as i32, dock_y, total_w as i32, dock_h, 16, 32, 32, 40, 215);
    // Draw Glass rim highlight
    draw_rounded_rect_outline_alpha(start_x as i32, dock_y, total_w as i32, dock_h, 16, 90, 95, 115, 1, 90);
    
    // ────────────────────────────────────────────────────────
    // 3. Render Dock Items
    // ────────────────────────────────────────────────────────
    for i in 0..5 {
        let size = sizes[i];
        let ix = xs[i] as i32;
        // Align to the bottom of the Dock with a 10px offset
        let iy = (sh - 16) - size as i32;
        
        // Render corresponding vector icon
        match i {
            0 => draw_vector_launchpad_icon(ix, iy, size as i32),
            1 => draw_vector_metrics_icon(ix, iy, size as i32),
            2 => draw_vector_folder_icon(ix, iy, size as i32),
            3 => draw_vector_terminal_icon(ix, iy, size as i32),
            4 => draw_vector_radiation_icon(ix, iy, size as i32),
            _ => {}
        }
        
        // Draw active/focused dots below icons
        // Item 0 is Launchpad (always available)
        // Items 1, 2, 3, 4 correspond to windows with IDs 0, 2, 1, 3
        let win_id = match i {
            1 => Some(0), // Metrics
            2 => Some(2), // Files
            3 => Some(1), // Console
            4 => Some(4), // Radiation Simulator
            _ => None,
        };
        
        if let Some(wid) = win_id {
            let mut is_open = false;
            let mut is_focused = false;
            unsafe {
                for idx in 0..5 {
                    if let Some(ref win) = WINDOWS[idx] {
                        if win.id == wid {
                            is_open = win.is_open;
                            is_focused = win.is_focused;
                            break;
                        }
                    }
                }
            }
            
            if is_open {
                // Active Dot
                let dot_y = sh - 11;
                let dot_x = ix + (size as i32 / 2) - 2;
                if is_focused {
                    // White active dot for focused app
                    draw_rounded_rect_alpha(dot_x, dot_y, 4, 4, 2, 255, 255, 255, 230);
                } else {
                    // Dim/translucent dot for unfocused open app
                    draw_rounded_rect_alpha(dot_x, dot_y, 4, 4, 2, 160, 170, 190, 150);
                }
            }
        }
    }
    
    // ────────────────────────────────────────────────────────
    // 4. Render Launchpad (Start Menu) if active
    // ────────────────────────────────────────────────────────
    let start_menu_open = START_MENU_OPEN.load(Ordering::Relaxed);
    let start_menu_animating = START_MENU_ANIMATING.load(Ordering::Relaxed);
    let progress = f32::from_bits(START_MENU_ANIM_PROGRESS.load(Ordering::Relaxed));
    if start_menu_open || start_menu_animating {
        draw_start_menu(cursor_x, cursor_y, dock_y, progress);
    }
}
