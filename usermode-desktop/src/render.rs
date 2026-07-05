// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Compositor Rendering Orchestration
//!
//! Orchestrates the full desktop redraw sequence, managing dirty rectangles,
//! drop-shadow layers, windows stack drawing, and the taskbar overlay.

use crate::state::{BACK_BUFFER, WINDOW_BACKING_STORES, SCREEN_WIDTH, SCREEN_HEIGHT, START_MENU_OPEN, START_MENU_ANIMATING};
use core::sync::atomic::Ordering;
use crate::graphics::shadow::draw_window_shadow;
use crate::graphics::compositor::{draw_cursor_to_buf, copy_rect_back_to_fb, snapshot_window_backing_store, restore_window_backing_store};
use crate::wallpaper::draw_wallpaper;
use crate::window::{draw_window, WINDOWS};
use crate::monitor::draw_monitor_window;
use crate::console::draw_console_window;
use crate::file_manager::draw_file_manager;
use crate::settings::draw_settings_window;
use crate::radiation::draw_radiation_window;
use crate::taskbar::{draw_taskbar, get_dock_layout};
use crate::dirty::DirtyRectTracker;
use crate::syscalls::{ScreenInfo, SharedSystemInfo};

/// Orchestrates the entire compositor frame draw pass.
pub fn draw_compositor_frame(
    screen_info: &ScreenInfo,
    dirty_tracker: &mut DirtyRectTracker,
    shared_info: *const SharedSystemInfo,
    cursor_x: i32,
    cursor_y: i32,
    prev_render_x: &mut i32,
    prev_render_y: &mut i32,
    ticks: u64,
    anim_running: bool,
    needs_redraw: &mut bool,
) {
    let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
    let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
    let fb_ptr = screen_info.framebuffer_addr as *mut u8;

    if *needs_redraw {
        // Blit the cached nebula wallpaper as the first layer
        draw_wallpaper();

        // Populate dirty tracker dynamically
        let is_anim = anim_running || START_MENU_ANIMATING.load(Ordering::Relaxed);
        if dirty_tracker.is_all_dirty() || is_anim {
            dirty_tracker.mark_all_dirty();
        } else {
            // 1. Ticks CPU/RAM telemetry text area at top-right
            dirty_tracker.add_rect(sw - 200, 0, 200, 30);
            
            // 2. Start menu Launchpad region if open or animating
            if START_MENU_OPEN.load(Ordering::Relaxed) || START_MENU_ANIMATING.load(Ordering::Relaxed) {
                let (_dock_start_x, _dock_w, dock_sizes, dock_xs) = get_dock_layout(sw, sh, cursor_x, cursor_y);
                let launchpad_cx = dock_xs[0] + dock_sizes[0] / 2.0;
                let menu_w = 220;
                let menu_h = 220;
                let menu_x = (launchpad_cx - menu_w as f32 / 2.0) as i32;
                let menu_y = (sh - 82) - menu_h - 12;
                dirty_tracker.add_rect(menu_x, menu_y, menu_w, menu_h);
            }
            
            // 3. Dock region
            let (dock_start_x, dock_w, _, _) = get_dock_layout(sw, sh, cursor_x, cursor_y);
            dirty_tracker.add_rect(dock_start_x as i32 - 20, sh - 100, dock_w as i32 + 40, 100);
            
            // 5. Windows that are open/animating/dirty
            unsafe {
                for i in 0..5 {
                    if let Some(ref win) = WINDOWS[i] {
                        if !win.is_open && !win.is_animating {
                            continue;
                        }
                        let (ax, ay) = win.get_animated_pos();
                        let store = &WINDOW_BACKING_STORES[win.id as usize];
                        if store.is_dirty.load(Ordering::Relaxed) || win.is_animating {
                            dirty_tracker.add_rect(ax - 20, ay - 20, win.width as i32 + 40, win.height as i32 + 40);
                        }
                    }
                }
            }
        }

        unsafe {
            for i in 0..5 {
                if let Some(ref win) = WINDOWS[i] {
                    if !win.is_open && !win.is_animating {
                        continue;
                    }
                    let (ax, ay) = win.get_animated_pos();
                    draw_window_shadow(ax, ay, win.width as i32, win.height as i32);
                    
                    let store = &WINDOW_BACKING_STORES[win.id as usize];
                    let is_maximized = win.is_maximized;
                    
                    if store.is_dirty.load(Ordering::Relaxed) || win.is_animating || is_maximized {
                        draw_window(win);
                        
                        if win.id == 0 {
                            draw_monitor_window(ax, ay, shared_info);
                        } else if win.id == 1 {
                            draw_console_window(ax, ay);
                        } else if win.id == 2 {
                            draw_file_manager(ax, ay, win.width, win.height);
                        } else if win.id == 3 {
                            draw_settings_window(ax, ay, win.width, win.height);
                        } else if win.id == 4 {
                            draw_radiation_window(ax, ay, win.width, win.height);
                        }
                        
                        if !win.is_animating && !is_maximized {
                            snapshot_window_backing_store(store, ax, ay, win.width, win.height);
                            store.is_dirty.store(false, Ordering::Relaxed);
                        }
                    } else {
                        restore_window_backing_store(store, ax, ay);
                    }
                }
            }

            // Render modular taskbar and start menu
            draw_taskbar(sw, sh, cursor_x, cursor_y, ticks, shared_info);
        }

        unsafe {
            if dirty_tracker.is_all_dirty() {
                let back_buffer_ptr = core::ptr::addr_of!(BACK_BUFFER.0) as *const u8;
                core::ptr::copy_nonoverlapping(
                    back_buffer_ptr,
                    fb_ptr,
                    (sw * sh * 3) as usize,
                );
            } else {
                // Restore background under old cursor from BACK_BUFFER
                copy_rect_back_to_fb(fb_ptr, *prev_render_x, *prev_render_y, 8, 12, sw, sh);
                
                for opt_rect in dirty_tracker.get_rects() {
                    if let Some(rect) = opt_rect {
                        copy_rect_back_to_fb(fb_ptr, rect.x, rect.y, rect.w, rect.h, sw, sh);
                    }
                }
            }
            
            // Draw cursor directly on framebuffer
            draw_cursor_to_buf(fb_ptr, cursor_x, cursor_y, sw, sh);
            *prev_render_x = cursor_x;
            *prev_render_y = cursor_y;
        }

        *needs_redraw = false;
        dirty_tracker.clear();
    } else if cursor_x != *prev_render_x || cursor_y != *prev_render_y {
        // Restore background under old cursor from BACK_BUFFER
        copy_rect_back_to_fb(fb_ptr, *prev_render_x, *prev_render_y, 8, 12, sw, sh);
        // Draw cursor at new position directly to framebuffer
        draw_cursor_to_buf(fb_ptr, cursor_x, cursor_y, sw, sh);
        *prev_render_x = cursor_x;
        *prev_render_y = cursor_y;
    }
}
