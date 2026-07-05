// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Shadow Rendering
//!
//! Provides soft dropshadow effects for windows and components. Includes clipping
//! region exclusion algorithms to prevent redundant drawing under window bodies.

use crate::state::SHADOWS_ENABLED;
use core::sync::atomic::Ordering;
use crate::graphics::core::{draw_pixel_alpha, draw_rect_alpha, get_corner_coverage};

/// Draws an alpha-blended color rectangle onto the back buffer while excluding a specified rectangular sub-region.
pub fn draw_rect_alpha_exclude(
    x: i32, y: i32, w: i32, h: i32,
    r: u8, g: u8, b: u8, alpha: u8,
    ex: i32, ey: i32, ew: i32, eh: i32
) {
    if w <= 0 || h <= 0 { return; }

    let x1 = x;
    let y1 = y;
    let x2 = x + w;
    let y2 = y + h;

    let ex1 = ex;
    let ey1 = ey;
    let ex2 = ex + ew;
    let ey2 = ey + eh;

    let ix1 = core::cmp::max(x1, ex1);
    let iy1 = core::cmp::max(y1, ey1);
    let ix2 = core::cmp::min(x2, ex2);
    let iy2 = core::cmp::min(y2, ey2);

    if ix1 >= ix2 || iy1 >= iy2 {
        draw_rect_alpha(x, y, w, h, r, g, b, alpha);
        return;
    }

    if iy1 > y1 {
        draw_rect_alpha(x1, y1, x2 - x1, iy1 - y1, r, g, b, alpha);
    }
    if iy2 < y2 {
        draw_rect_alpha(x1, iy2, x2 - x1, y2 - iy2, r, g, b, alpha);
    }
    if ix1 > x1 {
        draw_rect_alpha(x1, iy1, ix1 - x1, iy2 - iy1, r, g, b, alpha);
    }
    if ix2 < x2 {
        draw_rect_alpha(ix2, iy1, x2 - ix2, iy2 - iy1, r, g, b, alpha);
    }
}

/// Renders a rectangular shadow block with alpha blending, excluding the window interior.
pub fn draw_shadow_rect_alpha(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, alpha: u8, win_x: i32, win_y: i32, win_w: i32, win_h: i32) {
    draw_rect_alpha_exclude(x, y, w, h, r, g, b, alpha, win_x, win_y, win_w, win_h);
}

/// Draws a rounded shadow rectangle with alpha blending while excluding the window body.
pub fn draw_shadow_rounded_rect_alpha(
    x: i32, y: i32, w: i32, h: i32, radius: i32,
    r: u8, g: u8, b: u8, alpha: u8,
    win_x: i32, win_y: i32, win_w: i32, win_h: i32
) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    draw_shadow_rect_alpha(x + radius, y, w - 2 * radius, radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    draw_shadow_rect_alpha(x, y + radius, w, h - 2 * radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    draw_shadow_rect_alpha(x + radius, y + h - radius, w - 2 * radius, radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            let cov_tl = get_corner_coverage(radius, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                let blended = ((alpha as u32 * cov_tl as u32) / 255) as u8;
                let px = x + dx;
                let py = y + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            let cov_tr = get_corner_coverage(radius, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                let blended = ((alpha as u32 * cov_tr as u32) / 255) as u8;
                let px = x + w - radius + dx;
                let py = y + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            let cov_bl = get_corner_coverage(radius, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                let blended = ((alpha as u32 * cov_bl as u32) / 255) as u8;
                let px = x + dx;
                let py = y + h - radius + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            let cov_br = get_corner_coverage(radius, dx_f + 0.5, dy_f + 0.5);
            if cov_br > 0 {
                let blended = ((alpha as u32 * cov_br as u32) / 255) as u8;
                let px = x + w - radius + dx;
                let py = y + h - radius + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
        }
    }
}

/// Draws a soft multi-layer window shadow that is wider and more diffuse than a hard outline.
pub fn draw_window_shadow(win_x: i32, win_y: i32, win_w: i32, win_h: i32) {
    if !SHADOWS_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let shadow_shift_y = 3; 
    for d in (2..=10).step_by(2) {
        let ratio = d as f32 / 10.0;
        let alpha = (24.0 * (1.0 - ratio * ratio)) as u8;
        if alpha > 0 {
            draw_shadow_rounded_rect_alpha(
                win_x - d,
                win_y - d + shadow_shift_y,
                win_w + 2 * d,
                win_h + 2 * d,
                14 + d,
                0, 0, 0,
                alpha,
                win_x, win_y, win_w, win_h
            );
        }
    }
}
