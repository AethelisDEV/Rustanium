// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Compositor and Backing Store Caching
//!
//! Handles desktop screen blits, cursors rendering, launchpad menus overlays,
//! and window backing store snapshot capture and restore helpers.

use crate::state::{BACK_BUFFER, SCREEN_FORMAT, SCREEN_WIDTH, SCREEN_HEIGHT};
use core::sync::atomic::Ordering;
use crate::graphics::core::{draw_pixel, draw_rect_alpha, draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha};
use crate::atlas_font::{draw_text_atlas, measure_text, AtlasSize, AtlasWeight};

/// Draws default cursor onto BACK_BUFFER.
pub fn draw_cursor(cx: i32, cy: i32) {
    #[rustfmt::skip]
    const CURSOR_MAP: [[u8; 8]; 12] = [
        [1, 1, 0, 0, 0, 0, 0, 0],
        [1, 2, 1, 0, 0, 0, 0, 0],
        [1, 2, 2, 1, 0, 0, 0, 0],
        [1, 2, 2, 2, 1, 0, 0, 0],
        [1, 2, 2, 2, 2, 1, 0, 0],
        [1, 2, 2, 2, 2, 2, 1, 0],
        [1, 2, 2, 2, 2, 2, 2, 1],
        [1, 2, 2, 2, 2, 1, 1, 1],
        [1, 2, 2, 1, 2, 1, 0, 0],
        [1, 2, 1, 0, 1, 2, 1, 0],
        [1, 1, 0, 0, 0, 1, 1, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];

    for row in 0..12 {
        for col in 0..8 {
            let px = CURSOR_MAP[row][col];
            if px == 1 {
                draw_pixel(cx + col as i32, cy + row as i32, 0, 0, 0);
            } else if px == 2 {
                draw_pixel(cx + col as i32, cy + row as i32, 255, 255, 255);
            }
        }
    }
}

/// Renders cursor overlay directly on physical UEFI framebuffer pointer.
pub fn draw_cursor_to_buf(buf: *mut u8, cx: i32, cy: i32, sw: i32, sh: i32) {
    #[rustfmt::skip]
    const CURSOR_MAP: [[u8; 8]; 12] = [
        [1, 1, 0, 0, 0, 0, 0, 0],
        [1, 2, 1, 0, 0, 0, 0, 0],
        [1, 2, 2, 1, 0, 0, 0, 0],
        [1, 2, 2, 2, 1, 0, 0, 0],
        [1, 2, 2, 2, 2, 1, 0, 0],
        [1, 2, 2, 2, 2, 2, 1, 0],
        [1, 2, 2, 2, 2, 2, 2, 1],
        [1, 2, 2, 2, 2, 1, 1, 1],
        [1, 2, 2, 1, 2, 1, 0, 0],
        [1, 2, 1, 0, 1, 2, 1, 0],
        [1, 1, 0, 0, 0, 1, 1, 0],
        [0, 0, 0, 0, 0, 0, 0, 0],
    ];

    for row in 0..12 {
        for col in 0..8 {
            let px = CURSOR_MAP[row][col];
            let x = cx + col as i32;
            let y = cy + row as i32;
            if x >= 0 && x < sw && y >= 0 && y < sh {
                let idx = ((y * sw + x) * 3) as usize;
                if px == 1 {
                    unsafe {
                        *buf.add(idx) = 0;
                        *buf.add(idx + 1) = 0;
                        *buf.add(idx + 2) = 0;
                    }
                } else if px == 2 {
                    unsafe {
                        *buf.add(idx) = 255;
                        *buf.add(idx + 1) = 255;
                        *buf.add(idx + 2) = 255;
                    }
                }
            }
        }
    }
}

/// Blits a rectangular frame segment from BACK_BUFFER directly to GOP framebuffer.
pub fn copy_rect_back_to_fb(fb_ptr: *mut u8, rx: i32, ry: i32, rw: i32, rh: i32, sw: i32, sh: i32) {
    if rw <= 0 || rh <= 0 { return; }
    let start_y = core::cmp::max(0, ry);
    let end_y = core::cmp::min(sh, ry + rh);
    let start_x = core::cmp::max(0, rx);
    let end_x = core::cmp::min(sw, rx + rw);
    if start_x >= end_x || start_y >= end_y { return; }

    unsafe {
        let back_buf_ptr = core::ptr::addr_of!(BACK_BUFFER.0) as *const u8;
        for cy in start_y..end_y {
            let row_offset = (cy * sw) as usize;
            let src_row = back_buf_ptr.add((row_offset + start_x as usize) * 3);
            let dst_row = fb_ptr.add((row_offset + start_x as usize) * 3);
            let byte_count = ((end_x - start_x) * 3) as usize;
            core::ptr::copy_nonoverlapping(src_row, dst_row, byte_count);
        }
    }
}

/// Captures window pixel data and snapshots it to backing store.
pub fn snapshot_window_backing_store(store: &crate::state::WindowBackingStore, wx: i32, wy: i32, ww: usize, wh: usize) {
    if ww == 0 || wh == 0 { return; }
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let is_format_0 = SCREEN_FORMAT.load(Ordering::Relaxed) == 0;

    store.width.store(ww as u32, Ordering::Relaxed);
    store.height.store(wh as u32, Ordering::Relaxed);

    unsafe {
        let back_buf_ptr = core::ptr::addr_of!(BACK_BUFFER.0) as *const u8;
        for cy in 0..wh {
            let screen_y = wy + cy as i32;
            if screen_y < 0 || screen_y >= sh { continue; }
            let row_offset = (screen_y * sw) as usize;
            for cx in 0..ww {
                let screen_x = wx + cx as i32;
                if screen_x < 0 || screen_x >= sw { continue; }
                let idx = (row_offset + screen_x as usize) * 3;
                let (r, g, b) = if is_format_0 {
                    (*back_buf_ptr.add(idx + 2), *back_buf_ptr.add(idx + 1), *back_buf_ptr.add(idx))
                } else {
                    (*back_buf_ptr.add(idx), *back_buf_ptr.add(idx + 1), *back_buf_ptr.add(idx + 2))
                };
                let argb = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                let store_idx = cy * 580 + cx;
                if store_idx < store.pixels.len() {
                    store.pixels[store_idx].store(argb, Ordering::Relaxed);
                }
            }
        }
    }
}

/// Restores cached window pixel data back to BACK_BUFFER.
pub fn restore_window_backing_store(store: &crate::state::WindowBackingStore, wx: i32, wy: i32) {
    let ww = store.width.load(Ordering::Relaxed) as usize;
    let wh = store.height.load(Ordering::Relaxed) as usize;
    if ww == 0 || wh == 0 { return; }
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let is_format_0 = SCREEN_FORMAT.load(Ordering::Relaxed) == 0;

    unsafe {
        let back_buf_ptr = core::ptr::addr_of_mut!(BACK_BUFFER.0) as *mut u8;
        for cy in 0..wh {
            let screen_y = wy + cy as i32;
            if screen_y < 0 || screen_y >= sh { continue; }
            let row_offset = (screen_y * sw) as usize;
            for cx in 0..ww {
                let screen_x = wx + cx as i32;
                if screen_x < 0 || screen_x >= sw { continue; }
                let store_idx = cy * 580 + cx;
                if store_idx < store.pixels.len() {
                    let argb = store.pixels[store_idx].load(Ordering::Relaxed);
                    let r = ((argb >> 16) & 0xFF) as u8;
                    let g = ((argb >> 8) & 0xFF) as u8;
                    let b = (argb & 0xFF) as u8;

                    let idx = (row_offset + screen_x as usize) * 3;
                    if is_format_0 {
                        *back_buf_ptr.add(idx) = b;
                        *back_buf_ptr.add(idx + 1) = g;
                        *back_buf_ptr.add(idx + 2) = r;
                    } else {
                        *back_buf_ptr.add(idx) = r;
                        *back_buf_ptr.add(idx + 1) = g;
                        *back_buf_ptr.add(idx + 2) = b;
                    }
                }
            }
        }
    }
}

/// Represents descriptive metadata for an available system application in the grid.
pub struct AppInfo {
    /// Title displayed in the grid underneath the icon.
    pub title: &'static str,
    /// Bounding window identifier used to select and focus the application.
    pub id: u8,
    /// Numeric identifier mapping to specific icon drawing routines.
    pub icon_type: u8,
}

/// Static registry of all available applications in the user desktop environment.
pub const ALL_APPS: [AppInfo; 6] = [
    AppInfo { title: "System Monitor", id: 0, icon_type: 0 },
    AppInfo { title: "Console", id: 1, icon_type: 1 },
    AppInfo { title: "File Manager", id: 2, icon_type: 2 },
    AppInfo { title: "Settings", id: 3, icon_type: 3 },
    AppInfo { title: "Radiation Simulator", id: 4, icon_type: 4 },
    AppInfo { title: "Shut Down", id: 5, icon_type: 5 },
];

/// Checks if the query string is a case-insensitive substring of the title string.
pub fn matches_search(title: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q_bytes = query.as_bytes();
    let t_bytes = title.as_bytes();
    if q_bytes.len() > t_bytes.len() {
        return false;
    }

    for i in 0..=(t_bytes.len() - q_bytes.len()) {
        let mut matched = true;
        for j in 0..q_bytes.len() {
            let tc = t_bytes[i + j];
            let qc = q_bytes[j];
            let tc_lower = if tc >= b'A' && tc <= b'Z' { tc - b'A' + b'a' } else { tc };
            let qc_lower = if qc >= b'A' && qc <= b'Z' { qc - b'A' + b'a' } else { qc };
            if tc_lower != qc_lower {
                matched = false;
                break;
            }
        }
        if matched {
            return true;
        }
    }
    false
}

/// Computes the layout coordinates for an application card within the filtered list.
///
/// # Parameters
/// * `idx` - The 0-based index of the application in the filtered list.
/// * `matched_count` - The total number of applications matching the search query.
/// * `sw` - Screen width in pixels.
/// * `sh` - Screen height in pixels.
///
/// # Returns
/// `(x, y, w, h)` coordinates of the card.
pub fn get_app_card_layout(idx: usize, matched_count: usize, sw: i32, _sh: i32) -> (i32, i32, i32, i32) {
    let card_w = 160;
    let card_h = 140;
    let h_space = 40;
    let v_space = 30;
    let grid_y = 180;

    let r = idx / 3;
    let c = idx % 3;

    // Determine how many items are in the current row
    let row_start_idx = r * 3;
    let row_items = core::cmp::min(3, matched_count - row_start_idx);

    let total_row_w = row_items as i32 * card_w + (row_items as i32 - 1) * h_space;
    let row_start_x = (sw - total_row_w) / 2;

    let x = row_start_x + c as i32 * (card_w + h_space);
    let y = grid_y + r as i32 * (card_h + v_space);

    (x, y, card_w, card_h)
}

/// Renders the Launchpad overlay when sliding opened.
pub fn draw_start_menu(cursor_x: i32, cursor_y: i32, _tb_y: i32, progress: f32) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);

    // 1. Frosted Translucent Dark Backdrop
    let bg_alpha = (200.0 * progress) as u8;
    draw_rect_alpha(0, 24, sw, sh - 24, 12, 14, 20, bg_alpha);

    // 2. Search Bar
    let search_w = 360;
    let search_h = 42;
    let search_x = (sw - search_w) / 2;
    let search_y = 60;
    let search_radius = 21;

    let bar_alpha = (180.0 * progress) as u8;
    let outline_alpha = (100.0 * progress) as u8;

    draw_rounded_rect_alpha(search_x, search_y, search_w, search_h, search_radius, 45, 48, 62, bar_alpha);
    draw_rounded_rect_outline_alpha(search_x, search_y, search_w, search_h, search_radius, 90, 95, 115, 1, outline_alpha);

    // Get search query string
    let mut q_buf = [0u8; 64];
    let query_str = crate::state::SEARCH_QUERY.get_str(&mut q_buf);

    let text_x = search_x + 20;
    let text_y = search_y + (search_h - 14) / 2;

    if query_str.is_empty() {
        let placeholder = "Search applications...";
        let text_r = (120.0 * progress) as u8;
        let text_g = (125.0 * progress) as u8;
        let text_b = (140.0 * progress) as u8;
        draw_text_atlas(text_x, text_y, placeholder, text_r, text_g, text_b, AtlasSize::Small, AtlasWeight::Regular);
    } else {
        let text_r = (255.0 * progress) as u8;
        let text_g = (255.0 * progress) as u8;
        let text_b = (255.0 * progress) as u8;
        draw_text_atlas(text_x, text_y, query_str, text_r, text_g, text_b, AtlasSize::Small, AtlasWeight::Regular);

        // Blinking cursor
        let ticks = unsafe {
            let shared_info = crate::syscalls::sys_get_shared_info();
            (*shared_info).system_ticks.load(Ordering::Relaxed)
        };
        if (ticks / 30) % 2 == 0 {
            let qw = measure_text(query_str, AtlasSize::Small, AtlasWeight::Regular);
            let cur_x = text_x + qw + 2;
            let cur_y = text_y - 2;
            let cur_alpha = (230.0 * progress) as u8;
            draw_rect_alpha(cur_x, cur_y, 1, 18, 255, 255, 255, cur_alpha);
        }
    }

    // 3. Application Grid
    let mut matched_apps = [None; 6];
    let mut matched_count = 0;
    for app in ALL_APPS.iter() {
        if matches_search(app.title, query_str) {
            matched_apps[matched_count] = Some(app);
            matched_count += 1;
        }
    }

    if matched_count == 0 {
        let no_apps_text = "No applications found";
        let tw = measure_text(no_apps_text, AtlasSize::Medium, AtlasWeight::Regular);
        let tx = (sw - tw) / 2;
        let ty = 260;
        let tr = (160.0 * progress) as u8;
        let tg = (170.0 * progress) as u8;
        let tb = (185.0 * progress) as u8;
        draw_text_atlas(tx, ty, no_apps_text, tr, tg, tb, AtlasSize::Medium, AtlasWeight::Regular);
    } else {
        for idx in 0..matched_count {
            let app = matched_apps[idx].unwrap();
            let (cx, cy, cw, ch) = get_app_card_layout(idx, matched_count, sw, sh);

            // Hover check
            let hovered = cursor_x >= cx && cursor_x < cx + cw &&
                          cursor_y >= cy && cursor_y < cy + ch;

            if hovered {
                let card_alpha = (30.0 * progress) as u8;
                draw_rounded_rect_alpha(cx, cy, cw, ch, 12, 255, 255, 255, card_alpha);
                draw_rounded_rect_outline_alpha(cx, cy, cw, ch, 12, 255, 255, 255, 1, (40.0 * progress) as u8);
            }

            // Draw Application Vector Icon (scaled size 56x56)
            let icon_size = 56;
            let icon_x = cx + (cw - icon_size) / 2;
            let icon_y = cy + 20;

            match app.icon_type {
                0 => crate::graphics::draw_vector_metrics_icon(icon_x, icon_y, icon_size),
                1 => crate::graphics::draw_vector_terminal_icon(icon_x, icon_y, icon_size),
                2 => crate::graphics::draw_vector_folder_icon(icon_x, icon_y, icon_size),
                3 => crate::graphics::draw_vector_settings_icon(icon_x, icon_y, icon_size),
                4 => crate::graphics::draw_vector_radiation_icon(icon_x, icon_y, icon_size),
                5 => crate::graphics::draw_vector_shutdown_icon(icon_x, icon_y, icon_size),
                _ => {}
            }

            // Draw Label
            let label_weight = if hovered { AtlasWeight::SemiBold } else { AtlasWeight::Regular };
            let label_r = if hovered { (255.0 * progress) as u8 } else { (200.0 * progress) as u8 };
            let label_g = if hovered { (255.0 * progress) as u8 } else { (205.0 * progress) as u8 };
            let label_b = if hovered { (255.0 * progress) as u8 } else { (215.0 * progress) as u8 };

            let lw = measure_text(app.title, AtlasSize::Small, label_weight);
            let lx = cx + (cw - lw) / 2;
            let ly = cy + 92;

            draw_text_atlas(lx, ly, app.title, label_r, label_g, label_b, AtlasSize::Small, label_weight);
        }
    }
}

