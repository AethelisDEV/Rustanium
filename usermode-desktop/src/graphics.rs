// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

use crate::state::{BACK_BUFFER, SCREEN_FORMAT, SCREEN_WIDTH, SCREEN_HEIGHT};
use core::sync::atomic::Ordering;
use crate::font::FONT_8X16;

pub const CORNER_ALPHA_6: [[u8; 6]; 6] = [
    [0,   30,  150, 230, 255, 255],
    [30,  120, 240, 255, 255, 255],
    [150, 240, 255, 255, 255, 255],
    [230, 255, 255, 255, 255, 255],
    [255, 255, 255, 255, 255, 255],
    [255, 255, 255, 255, 255, 255],
];

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
        let (dest_r, dest_g, dest_b) = unsafe {
            let buffer = &BACK_BUFFER.0;
            if is_format_0 {
                (buffer[idx + 2], buffer[idx + 1], buffer[idx])
            } else {
                (buffer[idx], buffer[idx + 1], buffer[idx + 2])
            }
        };
        
        let alpha_u = alpha as u32;
        let inv_alpha = 255 - alpha_u;
        
        let blended_r = (((r as u32 * alpha_u) + (dest_r as u32 * inv_alpha)) / 255) as u8;
        let blended_g = (((g as u32 * alpha_u) + (dest_g as u32 * inv_alpha)) / 255) as u8;
        let blended_b = (((b as u32 * alpha_u) + (dest_b as u32 * inv_alpha)) / 255) as u8;
        
        unsafe {
            let buffer = &mut BACK_BUFFER.0;
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

pub fn draw_rect_outline(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, thickness: i32) {
    draw_rect(x, y, w, thickness, r, g, b);
    draw_rect(x, y + h - thickness, w, thickness, r, g, b);
    draw_rect(x, y, thickness, h, r, g, b);
    draw_rect(x + w - thickness, y, thickness, h, r, g, b);
}

// Optimized Rounded Rectangle fill (reduces pixel checks from O(W*H) to O(R^2))
pub fn draw_rounded_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8) {
    draw_rounded_rect_alpha(x, y, w, h, radius, r, g, b, 255);
}

fn get_corner_coverage(radius: i32, rx: f32, ry: f32) -> u8 {
    let dist = sqrt_approx(rx * rx + ry * ry);
    if dist <= (radius as f32 - 0.5) {
        255
    } else if dist >= (radius as f32 + 0.5) {
        0
    } else {
        ((radius as f32 + 0.5 - dist) * 255.0) as u8
    }
}

fn get_outline_coverage(radius: i32, thickness: i32, rx: f32, ry: f32) -> u8 {
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

pub fn draw_rounded_rect_alpha(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, alpha: u8) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    // Draw top rectangle (excluding corners)
    draw_rect_alpha(x + radius, y, w - 2 * radius, radius, r, g, b, alpha);
    
    // Draw middle rectangle (full width)
    draw_rect_alpha(x, y + radius, w, h - 2 * radius, r, g, b, alpha);
    
    // Draw bottom rectangle (excluding corners)
    draw_rect_alpha(x + radius, y + h - radius, w - 2 * radius, radius, r, g, b, alpha);
    
    // Draw the 4 corners using sub-pixel mathematical anti-aliasing
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            // Top-Left
            let cov_tl = get_corner_coverage(radius, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                draw_pixel_alpha(x + dx, y + dy, r, g, b, ((alpha as u32 * cov_tl as u32) / 255) as u8);
            }
            
            // Top-Right
            let cov_tr = get_corner_coverage(radius, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + dy, r, g, b, ((alpha as u32 * cov_tr as u32) / 255) as u8);
            }
            
            // Bottom-Left
            let cov_bl = get_corner_coverage(radius, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                draw_pixel_alpha(x + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_bl as u32) / 255) as u8);
            }
            
            // Bottom-Right
            let cov_br = get_corner_coverage(radius, dx_f + 0.5, dy_f + 0.5);
            if cov_br > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_br as u32) / 255) as u8);
            }
        }
    }
}

pub fn draw_rounded_rect_outline(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, thickness: i32) {
    draw_rounded_rect_outline_alpha(x, y, w, h, radius, r, g, b, thickness, 255);
}

pub fn draw_rounded_rect_outline_alpha(x: i32, y: i32, w: i32, h: i32, radius: i32, r: u8, g: u8, b: u8, thickness: i32, alpha: u8) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    // Draw outer borders (excluding corner segments)
    draw_rect_alpha(x + radius, y, w - 2 * radius, thickness, r, g, b, alpha);
    draw_rect_alpha(x + radius, y + h - thickness, w - 2 * radius, thickness, r, g, b, alpha);
    draw_rect_alpha(x, y + radius, thickness, h - 2 * radius, r, g, b, alpha);
    draw_rect_alpha(x + w - thickness, y + radius, thickness, h - 2 * radius, r, g, b, alpha);
    
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            // Top-Left
            let cov_tl = get_outline_coverage(radius, thickness, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                draw_pixel_alpha(x + dx, y + dy, r, g, b, ((alpha as u32 * cov_tl as u32) / 255) as u8);
            }
            
            // Top-Right
            let cov_tr = get_outline_coverage(radius, thickness, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + dy, r, g, b, ((alpha as u32 * cov_tr as u32) / 255) as u8);
            }
            
            // Bottom-Left
            let cov_bl = get_outline_coverage(radius, thickness, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                draw_pixel_alpha(x + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_bl as u32) / 255) as u8);
            }
            
            // Bottom-Right
            let cov_br = get_outline_coverage(radius, thickness, dx_f + 0.5, dy_f + 0.5);
            if cov_br > 0 {
                draw_pixel_alpha(x + w - radius + dx, y + h - radius + dy, r, g, b, ((alpha as u32 * cov_br as u32) / 255) as u8);
            }
        }
    }
}

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

/// Bresenham line drawing.
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

/// 2-pixel-thick line — used for smooth chart curves.
pub fn draw_line_thick(x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    draw_line(x0, y0, x1, y1, r, g, b);
    if dx >= dy {
        draw_line(x0, y0 + 1, x1, y1 + 1, r, g, b);
    } else {
        draw_line(x0 + 1, y0, x1 + 1, y1, r, g, b);
    }
}

/// Render a deep-space nebula wallpaper into WALLPAPER_CACHE.
/// Call once during init; every frame call draw_wallpaper() to blit it.
pub fn init_nebula_wallpaper() {
    // Base vertical gradient — deep indigo (top) to near-black navy (bottom)
    draw_gradient(13, 8, 40, 5, 5, 22);

    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);

    // Teal nebula cloud — upper-right quadrant
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

    // Violet nebula cloud — center-left area
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

    // Warm amber glow — bottom-right accent
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

    // Stars — deterministic PRNG, no system random needed
    let mut seed: u32 = 0xDEAD_C0DE_u32;
    for _ in 0..350 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sx = (seed >> 15) as i32 % sw;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sy = (seed >> 15) as i32 % (sh - 60);
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let bright: u8 = 80 + (seed >> 24) as u8 % 175;
        draw_pixel(sx, sy, bright, bright, bright);
        // Occasional soft halo on brighter stars
        if (seed >> 18) & 7 == 0 {
            draw_pixel_alpha(sx - 1, sy, bright, bright, bright, 50);
            draw_pixel_alpha(sx + 1, sy, bright, bright, bright, 50);
            draw_pixel_alpha(sx, sy - 1, bright, bright, bright, 50);
            draw_pixel_alpha(sx, sy + 1, bright, bright, bright, 50);
        }
    }

    // Snapshot BACK_BUFFER into WALLPAPER_CACHE for per-frame reuse
    let n = (sw * sh * 3) as usize;
    unsafe {
        let src = core::ptr::addr_of!(crate::BACK_BUFFER.0) as *const u8;
        let dst = core::ptr::addr_of_mut!(crate::WALLPAPER_CACHE.0) as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, n);
    }
}

/// Desktop sidebar icons — 48×48 effective size, smooth vector style.
pub fn draw_icon(id: u8, x: i32, y: i32) {
    if id == 0 {
        // Files — 48×48 modern folder
        // Tab
        draw_rounded_rect_alpha(x + 2,  y + 2,  22,  8, 4,  55, 155, 230, 255);
        // Body
        draw_rounded_rect_alpha(x,      y + 8,  44, 34, 7,  48, 142, 218, 255);
        // Specular top highlight
        draw_rounded_rect_alpha(x + 3,  y + 9,  38,  8, 3, 130, 210, 255,  45);
        // Inner bottom shadow for depth
        draw_rounded_rect_alpha(x + 2, y + 32,  40,  6, 3,  25,  90, 160,  60);
    } else if id == 1 {
        // Console — 48×48 dark window card
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 42, 8,  22,  24,  36, 255);
        // Title bar
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 14, 8,  38,  42,  60, 255);
        draw_rect_alpha(        x + 2, y + 8, 44,  8, 38,  42,  60, 255);
        // Three traffic-light dots
        draw_rounded_rect_alpha(x +  8, y + 5, 6, 6, 3, 232,  74,  58, 255);
        draw_rounded_rect_alpha(x + 18, y + 5, 6, 6, 3, 242, 186,  26, 255);
        draw_rounded_rect_alpha(x + 28, y + 5, 6, 6, 3,  56, 200,  90, 255);
        // Abstract prompt lines
        draw_rect_alpha(x +  8, y + 22, 20, 3,  61, 174, 233, 210);
        draw_rect_alpha(x +  8, y + 29, 28, 3, 200, 205, 215,  90);
        draw_rect_alpha(x +  8, y + 36, 16, 3, 200, 205, 215,  60);
    } else if id == 2 {
        // Metrics — 48×48 chart card
        draw_rounded_rect_alpha(x + 2, y + 2, 44, 42, 8,  26,  30,  46, 255);
        // Three bar chart pillars (varying heights)
        draw_rounded_rect_alpha(x +  6, y + 26, 8, 14, 3,  61, 174, 233, 255);
        draw_rounded_rect_alpha(x + 18, y + 16, 8, 24, 3,  90, 200, 150, 255);
        draw_rounded_rect_alpha(x + 30, y + 21, 8, 19, 3, 180, 100, 230, 255);
        // Baseline
        draw_rect_alpha(x + 4, y + 38, 38, 1, 255, 255, 255, 30);
    }
}

pub fn draw_tiny_folder_icon(x: i32, y: i32) {
    // Modern folder: tab + body + specular top stripe
    draw_rounded_rect_alpha(x + 1, y,      8,  4, 2,  61, 174, 233, 255);
    draw_rounded_rect_alpha(x,     y + 3, 16, 10, 3,  47, 140, 215, 255);
    draw_rounded_rect_alpha(x + 1, y + 4, 14,  3, 1, 120, 200, 255,  45);
}

pub fn draw_tiny_file_icon(x: i32, y: i32) {
    // Clean document: light body + folded corner + subtle rule lines
    draw_rounded_rect_alpha(x + 2, y,      12, 14, 2, 215, 220, 232, 255);
    draw_rounded_rect_alpha(x + 9, y,       5,  5, 1, 175, 180, 198, 255);
    draw_rect_alpha(x + 4, y + 5,  7, 1, 130, 135, 155, 210);
    draw_rect_alpha(x + 4, y + 8,  7, 1, 130, 135, 155, 180);
    draw_rect_alpha(x + 4, y + 11, 5, 1, 130, 135, 155, 150);
}


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

            // Subpixel 1: top-left (sx_left, sy_top)
            if sx_left >= 0 && sx_left < 8 && sy_top >= 0 && sy_top < 16 {
                if (glyph[sy_top as usize] & (0x80 >> sx_left)) != 0 { active_subpixels += 1; }
            }

            // Subpixel 2: top-right (sx_right, sy_top)
            if sx_right >= 0 && sx_right < 8 && sy_top >= 0 && sy_top < 16 {
                if (glyph[sy_top as usize] & (0x80 >> sx_right)) != 0 { active_subpixels += 1; }
            }

            // Subpixel 3: bottom-left (sx_left, sy_bottom)
            if sx_left >= 0 && sx_left < 8 && sy_bottom >= 0 && sy_bottom < 16 {
                if (glyph[sy_bottom as usize] & (0x80 >> sx_left)) != 0 { active_subpixels += 1; }
            }

            // Subpixel 4: bottom-right (sx_right, sy_bottom)
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

/// Draws an alpha-blended color rectangle onto the back buffer while excluding a specified rectangular sub-region.
///
/// This function acts as a geometric clipper, subtracting the exclusion region (typically the window body)
/// from the target drawing rectangle (typically the shadow area surrounding the window). By partitioning the 
/// remaining drawable space into up to 4 non-overlapping slices (top, bottom, left, right), it avoids looping
/// over or checking pixels inside the excluded region. This significantly enhances rendering performance.
///
/// # Arguments
///
/// * `x` - The starting X-coordinate of the target drawing rectangle.
/// * `y` - The starting Y-coordinate of the target drawing rectangle.
/// * `w` - The width of the target drawing rectangle.
/// * `h` - The height of the target drawing rectangle.
/// * `r` - The red color component (0-255).
/// * `g` - The green color component (0-255).
/// * `b` - The blue color component (0-255).
/// * `alpha` - The opacity component (0-255).
/// * `ex` - The starting X-coordinate of the exclusion rectangular region.
/// * `ey` - The starting Y-coordinate of the exclusion rectangular region.
/// * `ew` - The width of the exclusion rectangular region.
/// * `eh` - The height of the exclusion rectangular region.
///
/// # Constraints & Edge Cases
///
/// * If either the drawing width `w` or height `h` is non-positive, the function immediately exits.
/// * If the exclusion region does not overlap with the target drawing rectangle, the entire rectangle is drawn normally.
/// * If the exclusion region completely covers the target drawing rectangle, no drawing commands are executed.
/// * All coordinate calculations are checked against screen boundaries inside the downstream `draw_rect_alpha`.
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

    // Calculate intersection of target and exclusion rectangles
    let ix1 = core::cmp::max(x1, ex1);
    let iy1 = core::cmp::max(y1, ey1);
    let ix2 = core::cmp::min(x2, ex2);
    let iy2 = core::cmp::min(y2, ey2);

    // If there is no overlap/intersection, draw the entire rectangle normally
    if ix1 >= ix2 || iy1 >= iy2 {
        draw_rect_alpha(x, y, w, h, r, g, b, alpha);
        return;
    }

    // Split target rectangle into up to 4 non-overlapping slices excluding the intersection:
    // 1. Top slice (covers full width above the intersection)
    if iy1 > y1 {
        draw_rect_alpha(x1, y1, x2 - x1, iy1 - y1, r, g, b, alpha);
    }
    // 2. Bottom slice (covers full width below the intersection)
    if iy2 < y2 {
        draw_rect_alpha(x1, iy2, x2 - x1, y2 - iy2, r, g, b, alpha);
    }
    // 3. Left slice (covers the region to the left of the intersection, between Y bounds of the intersection)
    if ix1 > x1 {
        draw_rect_alpha(x1, iy1, ix1 - x1, iy2 - iy1, r, g, b, alpha);
    }
    // 4. Right slice (covers the region to the right of the intersection, between Y bounds of the intersection)
    if ix2 < x2 {
        draw_rect_alpha(ix2, iy1, x2 - ix2, iy2 - iy1, r, g, b, alpha);
    }
}

/// Renders a rectangular shadow block with alpha blending, excluding the window interior.
///
/// This serves as an adapter that delegates to `draw_rect_alpha_exclude`, which efficiently
/// draws only the shadow boundaries without processing the pixels occupied by the window body.
///
/// # Arguments
///
/// * `x` - The starting X-coordinate of the shadow rectangle.
/// * `y` - The starting Y-coordinate of the shadow rectangle.
/// * `w` - The width of the shadow rectangle.
/// * `h` - The height of the shadow rectangle.
/// * `r` - Red channel value (0-255).
/// * `g` - Green channel value (0-255).
/// * `b` - Blue channel value (0-255).
/// * `alpha` - Alpha opacity value (0-255).
/// * `win_x` - The X-coordinate of the window body.
/// * `win_y` - The Y-coordinate of the window body.
/// * `win_w` - The width of the window body.
/// * `win_h` - The height of the window body.
pub fn draw_shadow_rect_alpha(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8, alpha: u8, win_x: i32, win_y: i32, win_w: i32, win_h: i32) {
    draw_rect_alpha_exclude(x, y, w, h, r, g, b, alpha, win_x, win_y, win_w, win_h);
}

/// Draws a rounded shadow rectangle with alpha blending while excluding the window body.
///
/// This function constructs a complete rounded rectangle representing a shadow layer. It uses
/// `draw_shadow_rect_alpha` to draw the top, middle, and bottom strip slices, and then computes
/// anti-aliased sub-pixel coverage for the four rounded corners, ensuring the shadow does not bleed
/// into the window body.
///
/// # Arguments
///
/// * `x` - The starting X-coordinate of the shadow box.
/// * `y` - The starting Y-coordinate of the shadow box.
/// * `w` - The width of the shadow box.
/// * `h` - The height of the shadow box.
/// * `radius` - The corner radius of the shadow box.
/// * `r` - Red channel value (0-255).
/// * `g` - Green channel value (0-255).
/// * `b` - Blue channel value (0-255).
/// * `alpha` - Alpha opacity value (0-255).
/// * `win_x` - The X-coordinate of the window body.
/// * `win_y` - The Y-coordinate of the window body.
/// * `win_w` - The width of the window body.
/// * `win_h` - The height of the window body.
pub fn draw_shadow_rounded_rect_alpha(
    x: i32, y: i32, w: i32, h: i32, radius: i32,
    r: u8, g: u8, b: u8, alpha: u8,
    win_x: i32, win_y: i32, win_w: i32, win_h: i32
) {
    if w <= 0 || h <= 0 { return; }
    let radius = core::cmp::min(radius, core::cmp::min(w / 2, h / 2));
    
    // Draw top rectangle (excluding corners)
    draw_shadow_rect_alpha(x + radius, y, w - 2 * radius, radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    
    // Draw middle rectangle (full width)
    draw_shadow_rect_alpha(x, y + radius, w, h - 2 * radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    
    // Draw bottom rectangle (excluding corners)
    draw_shadow_rect_alpha(x + radius, y + h - radius, w - 2 * radius, radius, r, g, b, alpha, win_x, win_y, win_w, win_h);
    
    // Draw the 4 corners using sub-pixel mathematical anti-aliasing
    for dy in 0..radius {
        for dx in 0..radius {
            let r_f = radius as f32;
            let dx_f = dx as f32;
            let dy_f = dy as f32;
            
            // Top-Left
            let cov_tl = get_corner_coverage(radius, r_f - 0.5 - dx_f, r_f - 0.5 - dy_f);
            if cov_tl > 0 {
                let blended = ((alpha as u32 * cov_tl as u32) / 255) as u8;
                let px = x + dx;
                let py = y + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            // Top-Right
            let cov_tr = get_corner_coverage(radius, dx_f + 0.5, r_f - 0.5 - dy_f);
            if cov_tr > 0 {
                let blended = ((alpha as u32 * cov_tr as u32) / 255) as u8;
                let px = x + w - radius + dx;
                let py = y + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            // Bottom-Left
            let cov_bl = get_corner_coverage(radius, r_f - 0.5 - dx_f, dy_f + 0.5);
            if cov_bl > 0 {
                let blended = ((alpha as u32 * cov_bl as u32) / 255) as u8;
                let px = x + dx;
                let py = y + h - radius + dy;
                if !(px >= win_x && px < win_x + win_w && py >= win_y && py < win_y + win_h) {
                    draw_pixel_alpha(px, py, r, g, b, blended);
                }
            }
            
            // Bottom-Right
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
///
/// This function renders a smooth, macOS-style drop shadow behind the window by executing
/// concentric layers of shadows. To maintain high rendering frame rates, it employs a 5-layer
/// strategy (stepping by 2 from 2 to 10 pixels of shadow size) and adjusts the opacity profile 
/// to maintain a clean visual gradient.
///
/// # Arguments
///
/// * `win_x` - The starting X-coordinate of the window.
/// * `win_y` - The starting Y-coordinate of the window.
/// * `win_w` - The width of the window.
/// * `win_h` - The height of the window.
pub fn draw_window_shadow(win_x: i32, win_y: i32, win_w: i32, win_h: i32) {
    if !crate::state::SHADOWS_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    // 5 concentric layers instead of 10 for performance, while preserving visual depth.
    // Shifted slightly down to simulate a top-down light source.
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
                draw_pixel(cx + col as i32, cy + row as i32, 0, 0, 0); // Clean black border
            } else if px == 2 {
                draw_pixel(cx + col as i32, cy + row as i32, 255, 255, 255); // White interior
            }
        }
    }
}

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

    // Shadow
    draw_window_shadow(menu_x, menu_y, menu_w, menu_h);

    // Dark-glass body (#18181A with high alpha)
    draw_rounded_rect_alpha(menu_x, menu_y, menu_w, menu_h, radius, 24, 24, 28, 240);
    // 1px subtle border
    draw_rounded_rect_outline_alpha(menu_x, menu_y, menu_w, menu_h, radius, 70, 75, 95, 1, 120);

    // Header — Inter Regular, spaced out!
    let header_title = "L A U N C H P A D";
    let tw = crate::atlas_font::measure_text(header_title, crate::atlas_font::AtlasSize::Small, crate::atlas_font::AtlasWeight::Regular);
    let tx = menu_x + (menu_w - tw) / 2;
    crate::atlas_font::draw_text_atlas(
        tx, menu_y + 12,
        header_title,
        210, 220, 235,
        crate::atlas_font::AtlasSize::Small,
        crate::atlas_font::AtlasWeight::Regular,
    );
    // Separator
    draw_rect_alpha(menu_x + 12, menu_y + 34, menu_w - 24, 1, 60, 65, 80, 100);

    let items = ["System Monitor", "Files", "Console", "Settings", "Shut Down"];
    for (i, item) in items.iter().enumerate() {
        let iy      = menu_y + 44 + (i as i32) * 33;
        let hovered = cursor_x >= menu_x + 8 && cursor_x < menu_x + menu_w - 8 &&
                      cursor_y >= iy           && cursor_y < iy + 27;
        if hovered {
            draw_rounded_rect_alpha(menu_x + 8, iy, menu_w - 16, 27, 6, 61, 174, 233, 255);
            crate::atlas_font::draw_text_atlas(
                menu_x + 18, iy + 6,
                item,
                255, 255, 255,
                crate::atlas_font::AtlasSize::Small,
                crate::atlas_font::AtlasWeight::SemiBold,
            );
        } else {
            crate::atlas_font::draw_text_atlas(
                menu_x + 18, iy + 6,
                item,
                190, 200, 215,
                crate::atlas_font::AtlasSize::Small,
                crate::atlas_font::AtlasWeight::Regular,
            );
        }
    }
}

pub fn draw_vector_launchpad_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    // Card - modern glass
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 50, 50, 60, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 100, 100, 120, 1, 100);
    // Grid of 9 dots
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

pub fn draw_vector_folder_icon(x: i32, y: i32, size: i32) {
    let r_tab = (size as f32 * 0.08) as i32;
    let r_body = (size as f32 * 0.15) as i32;
    // Tab
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.05) as i32,
        y + (size as f32 * 0.05) as i32,
        (size as f32 * 0.5) as i32,
        (size as f32 * 0.2) as i32,
        r_tab.max(2),
        55, 155, 230, 255
    );
    // Body
    draw_rounded_rect_alpha(
        x,
        y + (size as f32 * 0.18) as i32,
        size,
        (size as f32 * 0.82) as i32,
        r_body.max(3),
        48, 142, 218, 255
    );
    // Highlight
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.08) as i32,
        y + (size as f32 * 0.22) as i32,
        (size as f32 * 0.84) as i32,
        (size as f32 * 0.15) as i32,
        r_tab.max(1),
        130, 210, 255, 45
    );
}

pub fn draw_vector_terminal_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    // Card
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 20, 22, 34, 255);
    // Rim highlight
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 60, 65, 80, 1, 100);
    // Three dots
    let dot_size = (size as f32 * 0.08) as i32;
    let dot_y = y + (size as f32 * 0.15) as i32;
    let r_dot = (dot_size as f32 * 0.5) as i32;
    draw_rounded_rect_alpha(x + (size as f32 * 0.15) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 230, 72, 58, 255);
    draw_rounded_rect_alpha(x + (size as f32 * 0.3) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 240, 185, 25, 255);
    draw_rounded_rect_alpha(x + (size as f32 * 0.45) as i32, dot_y, dot_size.max(2), dot_size.max(2), r_dot.max(1), 39, 201, 63, 255);
    // terminal text cursor prompt
    let px = x + (size as f32 * 0.25) as i32;
    let py = y + (size as f32 * 0.5) as i32;
    let pw = (size as f32 * 0.15) as i32;
    draw_line_thick(px, py, px + pw, py + pw / 2, 230, 235, 250);
    draw_line_thick(px + pw, py + pw / 2, px, py + pw, 230, 235, 250);
}

pub fn draw_vector_metrics_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    // Card
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 32, 36, 54, 255);
    // Rim highlight
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 80, 85, 105, 1, 80);
    
    // 3 bars
    let bar_w = (size as f32 * 0.18) as i32;
    let bar_r = (bar_w as f32 * 0.3) as i32;
    // Bar 1
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.18) as i32,
        y + (size as f32 * 0.5) as i32,
        bar_w.max(2),
        (size as f32 * 0.35) as i32,
        bar_r.max(1),
        61, 174, 233, 255
    );
    // Bar 2
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.41) as i32,
        y + (size as f32 * 0.25) as i32,
        bar_w.max(2),
        (size as f32 * 0.6) as i32,
        bar_r.max(1),
        90, 200, 150, 255
    );
    // Bar 3
    draw_rounded_rect_alpha(
        x + (size as f32 * 0.64) as i32,
        y + (size as f32 * 0.4) as i32,
        bar_w.max(2),
        (size as f32 * 0.45) as i32,
        bar_r.max(1),
        180, 100, 230, 255
    );
}

/// Draws a modern vector slider-control/settings icon.
///
/// This icon consists of a dark background containing three horizontal slider tracks with active knobs,
/// representing modern desktop performance configuration controls.
///
/// # Arguments
///
/// * `x` - The starting X-coordinate of the icon.
/// * `y` - The starting Y-coordinate of the icon.
/// * `size` - The size of the icon bounding box.
pub fn draw_vector_settings_icon(x: i32, y: i32, size: i32) {
    let r = (size as f32 * 0.15) as i32;
    // Base card
    draw_rounded_rect_alpha(x, y, size, size, r.max(3), 44, 48, 64, 255);
    draw_rounded_rect_outline_alpha(x, y, size, size, r.max(3), 110, 115, 140, 1, 100);
    
    // Sliders
    let track_w = (size as f32 * 0.6) as i32;
    let track_x = x + (size as f32 * 0.2) as i32;
    let start_y = y + (size as f32 * 0.25) as i32;
    let step_y = (size as f32 * 0.25) as i32;
    
    for i in 0..3 {
        let ty = start_y + i * step_y;
        // Track line
        draw_rounded_rect_alpha(track_x, ty, track_w, 2, 1, 80, 85, 105, 180);
        
        let knob_offset = match i {
            0 => (size as f32 * 0.3) as i32,
            1 => (size as f32 * 0.5) as i32,
            2 => (size as f32 * 0.2) as i32,
            _ => 0,
        };
        let knob_x = track_x + knob_offset;
        // Active knob
        draw_rounded_rect_alpha(knob_x - 4, ty - 3, 8, 8, 4, 61, 174, 233, 255);
    }
}

