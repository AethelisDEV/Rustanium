// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Core Graphics Rendering
//!
//! Provides the primary primitive drawing functions (pixels, lines, rectangles,
//! curves, gradients) and base mathematical approximations (square root, arctangent).

use crate::state::{BACK_BUFFER, SCREEN_FORMAT, SCREEN_WIDTH, SCREEN_HEIGHT};
use core::sync::atomic::Ordering;

/// Transparency lookup table for rounded corner rendering.
pub const CORNER_ALPHA_6: [[u8; 6]; 6] = [
    [0,   30,  150, 230, 255, 255],
    [30,  120, 240, 255, 255, 255],
    [150, 240, 255, 255, 255, 255],
    [230, 255, 255, 255, 255, 255],
    [255, 255, 255, 255, 255, 255],
    [255, 255, 255, 255, 255, 255],
];

/// Draws a single pixel directly onto `BACK_BUFFER` with color formatting.
pub fn draw_pixel(x: i32, y: i32, r: u8, g: u8, b: u8) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    if x >= 0 && x < sw && y >= 0 && y < sh {
        let idx = ((y * sw + x) * 3) as usize;
        let is_format_0 = SCREEN_FORMAT.load(Ordering::Relaxed) == 0;
        unsafe {
            let buffer = &mut BACK_BUFFER.0;
            if is_format_0 {
                buffer[idx] = b;
                buffer[idx + 1] = g;
                buffer[idx + 2] = r;
            } else {
                buffer[idx] = r;
                buffer[idx + 1] = g;
                buffer[idx + 2] = b;
            }
        }
    }
}

/// Blends a pixel color with the background at the target coordinate using alpha channels.
pub fn draw_pixel_alpha(x: i32, y: i32, r: u8, g: u8, b: u8, alpha: u8) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    if x >= 0 && x < sw && y >= 0 && y < sh {
        if alpha == 0 {
            return;
        }
        if alpha == 255 {
            draw_pixel(x, y, r, g, b);
            return;
        }
        
        let idx = ((y * sw + x) * 3) as usize;
        let is_format_0 = SCREEN_FORMAT.load(Ordering::Relaxed) == 0;
        let alpha_u = alpha as u32;
        let inv_alpha = 255 - alpha_u;
        
        unsafe {
            let buffer = &mut BACK_BUFFER.0;
            let (dest_r, dest_g, dest_b) = if is_format_0 {
                (buffer[idx + 2], buffer[idx + 1], buffer[idx])
            } else {
                (buffer[idx], buffer[idx + 1], buffer[idx + 2])
            };
            
            let vr = r as u32 * alpha_u + dest_r as u32 * inv_alpha;
            let vg = g as u32 * alpha_u + dest_g as u32 * inv_alpha;
            let vb = b as u32 * alpha_u + dest_b as u32 * inv_alpha;
            
            let blended_r = ((vr + 1 + (vr >> 8)) >> 8) as u8;
            let blended_g = ((vg + 1 + (vg >> 8)) >> 8) as u8;
            let blended_b = ((vb + 1 + (vb >> 8)) >> 8) as u8;
            
            if is_format_0 {
                buffer[idx] = blended_b;
                buffer[idx + 1] = blended_g;
                buffer[idx + 2] = blended_r;
            } else {
                buffer[idx] = blended_r;
                buffer[idx + 1] = blended_g;
                buffer[idx + 2] = blended_b;
            }
        }
    }
}

/// Fills a solid rectangle on the screen.
pub fn draw_rect(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    if w <= 0 || h <= 0 { return; }
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let start_y = core::cmp::max(0, y);
    let end_y = core::cmp::min(sh, y + h);
    let start_x = core::cmp::max(0, x);
    let end_x = core::cmp::min(sw, x + w);
    if start_x >= end_x || start_y >= end_y { return; }
    
    let is_bgr = SCREEN_FORMAT.load(Ordering::Relaxed) == 0;
    unsafe {
        let dest_ptr = core::ptr::addr_of_mut!(BACK_BUFFER.0) as *mut u8;
        for cy in start_y..end_y {
            let row_offset = (cy * sw) as usize;
            for cx in start_x..end_x {
                let pixel_offset = (row_offset + cx as usize) * 3;
                if is_bgr {
                    *dest_ptr.add(pixel_offset) = b;
                    *dest_ptr.add(pixel_offset + 1) = g;
                    *dest_ptr.add(pixel_offset + 2) = r;
                } else {
                    *dest_ptr.add(pixel_offset) = r;
                    *dest_ptr.add(pixel_offset + 1) = g;
                    *dest_ptr.add(pixel_offset + 2) = b;
                }
            }
        }
    }
}

/// Fills an alpha-blended rectangle.
pub fn draw_rect_alpha(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, alpha: u8) {
    if w <= 0 || h <= 0 { return; }
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let start_y = core::cmp::max(0, y);
    let end_y = core::cmp::min(sh, y + h);
    let start_x = core::cmp::max(0, x);
    let end_x = core::cmp::min(sw, x + w);
    if start_x >= end_x || start_y >= end_y { return; }

    for cy in start_y..end_y {
        for cx in start_x..end_x {
            draw_pixel_alpha(cx, cy, r, g, b, alpha);
        }
    }
}

/// Draws a hollow rectangle outline.
pub fn draw_rect_outline(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, thickness: i32) {
    draw_rect(x, y, w, thickness, r, g, b);
    draw_rect(x, y + h - thickness, w, thickness, r, g, b);
    draw_rect(x, y, thickness, h, r, g, b);
    draw_rect(x + w - thickness, y, thickness, h, r, g, b);
}

/// Draws a solid rounded rectangle with default opacity.
pub fn draw_rounded_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8) {
    draw_rounded_rect_alpha(x, y, w, h, radius, r, g, b, 255);
}

/// Utility calculating pixel coverage for rounded corners.
pub fn get_corner_coverage(radius: i32, rx: f32, ry: f32) -> u8 {
    let dist = sqrt_approx(rx * rx + ry * ry);
    if dist <= (radius as f32 - 0.5) {
        255
    } else if dist >= (radius as f32 + 0.5) {
        0
    } else {
        ((radius as f32 + 0.5 - dist) * 255.0) as u8
    }
}

/// Utility calculating pixel coverage for outlines.
pub fn get_outline_coverage(radius: i32, thickness: i32, rx: f32, ry: f32) -> u8 {
    let dist = sqrt_approx(rx * rx + ry * ry);
    let inner_limit = (radius - thickness) as f32;
    let outer_limit = radius as f32;
    if dist >= outer_limit + 0.5 || dist <= inner_limit - 0.5 {
        0
    } else {
        let mut cov = 1.0f32;
        if dist > outer_limit - 0.5 {
            cov *= outer_limit + 0.5 - dist;
        }
        if dist < inner_limit + 0.5 {
            cov *= dist - (inner_limit - 0.5);
        }
        (cov.clamp(0.0, 1.0) * 255.0) as u8
    }
}

/// Draws an alpha-blended solid rounded rectangle.
pub fn draw_rounded_rect_alpha(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, alpha: u8) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    draw_rect_alpha(x + radius, y, w - 2 * radius, radius, r, g, b, alpha);
    draw_rect_alpha(x, y + radius, w, h - 2 * radius, r, g, b, alpha);
    draw_rect_alpha(x + radius, y + h - radius, w - 2 * radius, radius, r, g, b, alpha);
    
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            let cov_tl = get_corner_coverage(radius, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                draw_pixel_alpha(x + dx, y + dy, r, g, b, ((alpha as u32 * cov_tl as u32) / 255) as u8);
            }
            
            let cov_tr = get_corner_coverage(radius, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + dy, r, g, b, ((alpha as u32 * cov_tr as u32) / 255) as u8);
            }
            
            let cov_bl = get_corner_coverage(radius, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                draw_pixel_alpha(x + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_bl as u32) / 255) as u8);
            }
            
            let cov_br = get_corner_coverage(radius, dx_f + 0.5, dy_f + 0.5);
            if cov_br > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_br as u32) / 255) as u8);
            }
        }
    }
}

/// Draws a hollow rounded rectangle outline.
pub fn draw_rounded_rect_outline(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, thickness: i32) {
    draw_rounded_rect_outline_alpha(x, y, w, h, radius, r, g, b, thickness, 255);
}

/// Draws an alpha-blended hollow rounded rectangle outline.
pub fn draw_rounded_rect_outline_alpha(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, thickness: i32, alpha: u8) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    draw_rect_alpha(x + radius, y, w - 2 * radius, thickness, r, g, b, alpha);
    draw_rect_alpha(x + radius, y + h - thickness, w - 2 * radius, thickness, r, g, b, alpha);
    draw_rect_alpha(x, y + radius, thickness, h - 2 * radius, r, g, b, alpha);
    draw_rect_alpha(x + w - thickness, y + radius, thickness, h - 2 * radius, r, g, b, alpha);
    
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            let cov_tl = get_outline_coverage(radius, thickness, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                draw_pixel_alpha(x + dx, y + dy, r, g, b, ((alpha as u32 * cov_tl as u32) / 255) as u8);
            }
            
            let cov_tr = get_outline_coverage(radius, thickness, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + dy, r, g, b, ((alpha as u32 * cov_tr as u32) / 255) as u8);
            }
            
            let cov_bl = get_outline_coverage(radius, thickness, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                draw_pixel_alpha(x + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_bl as u32) / 255) as u8);
            }
            
            let cov_br = get_outline_coverage(radius, thickness, dx_f + 0.5, dy_f + 0.5);
            if cov_br > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_br as u32) / 255) as u8);
            }
        }
    }
}

/// Approximate square root function using Newton-Raphson method.
pub fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x;
    for _ in 0..6 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}

/// Trigonometric arctangent approximation.
pub fn atan_approx(z: f32) -> f32 {
    let z_abs = if z < 0.0 { -z } else { z };
    if z_abs < 1.0 {
        z / (1.0 + 0.28 * z * z)
    } else {
        let sign = if z < 0.0 { -1.0 } else { 1.0 };
        let inv_z = 1.0 / z;
        sign * 1.5707963 - inv_z / (1.0 + 0.28 * inv_z * inv_z)
    }
}

/// Trigonometric 2-argument arctangent approximation.
pub fn atan2_approx(y: f32, x: f32) -> f32 {
    if x > 0.0 {
        atan_approx(y / x)
    } else if x < 0.0 {
        if y >= 0.0 {
            atan_approx(y / x) + 3.14159265
        } else {
            atan_approx(y / x) - 3.14159265
        }
    } else {
        if y > 0.0 {
            1.5707963
        } else if y < 0.0 {
            -1.5707963
        } else {
            0.0
        }
    }
}

/// Renders a line segment using Bresenham's algorithm.
pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1i32 } else { -1i32 };
    let sy = if y0 < y1 { 1i32 } else { -1i32 };
    let mut err = dx - dy;
    let mut cx = x0;
    let mut cy = y0;
    loop {
        draw_pixel(cx, cy, r, g, b);
        if cx == x1 && cy == y1 { break; }
        let e2 = err * 2;
        if e2 > -dy { err -= dy; cx += sx; }
        if e2 <  dx { err += dx; cy += sy; }
    }
}

/// Renders a thick line segment for smooth graphs/curves.
pub fn draw_line_thick(x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1i32 } else { -1i32 };
    let sy = if y0 < y1 { 1i32 } else { -1i32 };
    let mut err = dx - dy;
    let mut cx = x0;
    let mut cy = y0;
    loop {
        draw_pixel(cx, cy, r, g, b);
        if dx >= dy {
            draw_pixel(cx, cy + 1, r, g, b);
        } else {
            draw_pixel(cx + 1, cy, r, g, b);
        }
        if cx == x1 && cy == y1 { break; }
        let e2 = err * 2;
        if e2 > -dy { err -= dy; cx += sx; }
        if e2 <  dx { err += dx; cy += sy; }
    }
}

/// Fills the screen with a vertical linear gradient.
pub fn draw_gradient(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    for y in 0..sh {
        let r = r1 as i32 + ((r2 as i32 - r1 as i32) * y) / sh;
        let g = g1 as i32 + ((g2 as i32 - g1 as i32) * y) / sh;
        let b = b1 as i32 + ((b2 as i32 - b1 as i32) * y) / sh;

        for x in 0..sw {
            draw_pixel(x, y, r as u8, g as u8, b as u8);
        }
    }
}

/// Renders a deep-space nebula wallpaper into `WALLPAPER_CACHE`.
pub fn init_nebula_wallpaper() {
    draw_gradient(13, 8, 40, 5, 5, 22);

    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);

    for y in 0..(sh * 2 / 3) {
        let cy = sh / 5;
        let rel_y = (y - cy).abs();
        let band = sh / 3;
        if rel_y >= band { continue; }
        let fy = 1.0 - rel_y as f32 / band as f32;
        for x in (sw / 3)..sw {
            let rel_x = (x - sw / 2).max(0);
            let fade_x = (rel_x as f32 / (sw as f32 / 2.5)).min(1.0);
            let a = (fy * fade_x * 55.0) as u8;
            if a > 1 { draw_pixel_alpha(x, y, 14, 118, 172, a); }
        }
    }

    for y in (sh / 5)..(sh * 4 / 5) {
        let cy = sh / 2;
        let rel_y = (y - cy).abs();
        let band = sh * 2 / 5;
        if rel_y >= band { continue; }
        let fy = 1.0 - rel_y as f32 / band as f32;
        for x in 0..(sw * 2 / 3) {
            let fade_x = 1.0 - x as f32 / (sw as f32 * 2.0 / 3.0);
            let a = (fy * fade_x * 48.0) as u8;
            if a > 1 { draw_pixel_alpha(x, y, 88, 28, 142, a); }
        }
    }

    for y in (sh / 2)..sh {
        let cy = sh * 3 / 4;
        let rel_y = (y - cy).abs();
        let band = sh / 5;
        if rel_y >= band { continue; }
        let fy = 1.0 - rel_y as f32 / band as f32;
        for x in (sw * 3 / 5)..sw {
            let fade_x = (x - sw * 3 / 5) as f32 / (sw as f32 * 2.0 / 5.0);
            let a = (fy * fade_x * 32.0) as u8;
            if a > 1 { draw_pixel_alpha(x, y, 182, 78, 18, a); }
        }
    }

    let mut seed: u32 = 0xDEAD_C0DE_u32;
    for _ in 0..350 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sx = (seed >> 15) as i32 % sw;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sy = (seed >> 15) as i32 % (sh - 60);
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let bright: u8 = 80 + (seed >> 24) as u8 % 175;
        draw_pixel(sx, sy, bright, bright, bright);
        if (seed >> 18) & 7 == 0 {
            draw_pixel_alpha(sx - 1, sy, bright, bright, bright, 50);
            draw_pixel_alpha(sx + 1, sy, bright, bright, bright, 50);
            draw_pixel_alpha(sx, sy - 1, bright, bright, bright, 50);
            draw_pixel_alpha(sx, sy + 1, bright, bright, bright, 50);
        }
    }

    let n = (sw * sh * 3) as usize;
    unsafe {
        let src = core::ptr::addr_of!(crate::state::BACK_BUFFER.0) as *const u8;
        let dst = core::ptr::addr_of_mut!(crate::state::WALLPAPER_CACHE.0) as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, n);
    }
}
