// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Compositor and Backing Store Caching
//!
//! Handles desktop screen blits, cursors rendering, launchpad menus overlays,
//! and window backing store snapshot capture and restore helpers.

use crate::state::{BACK_BUFFER, SCREEN_FORMAT, SCREEN_WIDTH, SCREEN_HEIGHT};
use core::sync::atomic::Ordering;
use crate::graphics::core::{draw_pixel, draw_rect_alpha, draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha};
use crate::graphics::shadow::{draw_window_shadow};
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

/// Renders the Launchpad overlay when sliding opened.
pub fn draw_start_menu(cursor_x: i32, cursor_y: i32, tb_y: i32, progress: f32) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let (_dock_start_x, _dock_w, _sizes, xs) = crate::taskbar::get_dock_layout(sw, sh, cursor_x, cursor_y);
    let launchpad_cx = xs[0] + _sizes[0] / 2.0;
    let menu_w = 220i32;
    let menu_h = 220i32;
    let menu_x = (launchpad_cx - menu_w as f32 / 2.0) as i32;
    let radius = 12;

    let full_y = tb_y - menu_h - 12;
    let menu_y = (tb_y as f32 + (full_y - tb_y) as f32 * progress) as i32;

    draw_window_shadow(menu_x, menu_y, menu_w, menu_h);
    draw_rounded_rect_alpha(menu_x, menu_y, menu_w, menu_h, radius, 24, 24, 28, 240);
    draw_rounded_rect_outline_alpha(menu_x, menu_y, menu_w, menu_h, radius, 70, 75, 95, 1, 120);

    let header_title = "L A U N C H P A D";
    let tw = measure_text(header_title, AtlasSize::Small, AtlasWeight::Regular);
    let tx = menu_x + (menu_w - tw) / 2;
    draw_text_atlas(
        tx, menu_y + 12,
        header_title,
        210, 220, 235,
        AtlasSize::Small,
        AtlasWeight::Regular,
    );
    draw_rect_alpha(menu_x + 12, menu_y + 34, menu_w - 24, 1, 60, 65, 80, 100);

    let items = ["System Monitor", "Files", "Console", "Settings", "Shut Down"];
    for (i, item) in items.iter().enumerate() {
        let iy      = menu_y + 44 + (i as i32) * 33;
        let hovered = cursor_x >= menu_x + 8 && cursor_x < menu_x + menu_w - 8 &&
                      cursor_y >= iy           && cursor_y < iy + 27;
        if hovered {
            draw_rounded_rect_alpha(menu_x + 8, iy, menu_w - 16, 27, 6, 61, 174, 233, 255);
            draw_text_atlas(
                menu_x + 18, iy + 6,
                item,
                255, 255, 255,
                AtlasSize::Small,
                AtlasWeight::SemiBold,
            );
        } else {
            draw_text_atlas(
                menu_x + 18, iy + 6,
                item,
                190, 200, 215,
                AtlasSize::Small,
                AtlasWeight::Regular,
            );
        }
    }
}
