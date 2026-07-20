// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # File Manager Input & Interaction Handlers
//!
//! Processes mouse clicks, mouse dragging, scrollbar interactions, modal button submissions,
//! and directory navigation events.

use core::sync::atomic::Ordering;
use crate::syscalls::{sys_open, sys_read, sys_close};
use super::state::{FILE_MANAGER_STATE, execute_modal_action};

/// Processes mouse dragging events targeted at the File Manager window body.
///
/// # Arguments
///
/// * `dy` - Mouse vertical movement delta.
/// * `cursor_x` - Active screen X-coordinate of mouse cursor.
/// * `cursor_y` - Active screen Y-coordinate of mouse cursor.
/// * `ax` - Animated starting X-coordinate of the window body.
/// * `ay` - Animated starting Y-coordinate of the window body.
/// * `win_width` - Active width of the File Manager window.
/// * `win_height` - Active height of the File Manager window.
pub fn handle_file_manager_mouse_drag(
    dy: i32,
    cursor_x: i32,
    cursor_y: i32,
    ax: i32,
    ay: i32,
    win_width: usize,
    win_height: usize,
) {
    let rx = cursor_x - ax;
    let ry = cursor_y - ay;
    let list_width = (win_width as i32 - 170).max(180);
    let container_h = win_height as i32 - 106;

    if ry >= 76 && ry < win_height as i32 - 30 && rx >= 12 && rx <= list_width + 16 {
        if dy < 0 {
            FILE_MANAGER_STATE.scroll_down(-dy * 2, container_h, list_width);
        } else if dy > 0 {
            FILE_MANAGER_STATE.scroll_up(dy * 2);
        }
    }
}

/// Processes mouse click events targeted at the File Manager window body.
///
/// Handles toolbar actions, view mode switching, scrollbar clicks, modal buttons,
/// item selections, and directory navigation.
///
/// # Arguments
///
/// * `cursor_x` - The active screen X-coordinate of the mouse click.
/// * `cursor_y` - The active screen Y-coordinate of the mouse click.
/// * `ax` - Animated starting X-coordinate of the window body.
/// * `ay` - Animated starting Y-coordinate of the window body.
/// * `win_width` - Active width of the File Manager window.
/// * `win_height` - Active height of the File Manager window.
pub fn handle_file_manager_click(
    cursor_x: i32,
    cursor_y: i32,
    ax: i32,
    ay: i32,
    win_width: usize,
    win_height: usize,
) {
    let rx = cursor_x - ax;
    let ry = cursor_y - ay;

    let modal_mode = FILE_MANAGER_STATE.modal_mode.load(Ordering::Relaxed);
    if modal_mode > 0 {
        let mw = 280i32;
        let mh = 140i32;
        let mx = ax + (win_width as i32 - mw) / 2;
        let my = ay + (win_height as i32 - mh) / 2;

        // Check Cancel Button
        if cursor_y >= my + 92 && cursor_y <= my + 120 && cursor_x >= mx + 104 && cursor_x <= mx + 174 {
            FILE_MANAGER_STATE.close_modal();
            return;
        }
        // Check Action/OK Button
        if cursor_y >= my + 92 && cursor_y <= my + 120 && cursor_x >= mx + 184 && cursor_x <= mx + 264 {
            execute_modal_action();
            return;
        }
        return;
    }

    // Check Toolbar Button Clicks (ry 38..68)
    if ry >= 38 && ry <= 68 {
        if rx >= 14 && rx <= 30 {
            // Back Button (<)
            FILE_MANAGER_STATE.pop_dir();
            return;
        } else if rx >= 34 && rx <= 50 {
            // Up / Root Button (^)
            FILE_MANAGER_STATE.set_path("/");
            return;
        } else if rx >= 54 && rx <= 70 {
            // Refresh Button (R)
            FILE_MANAGER_STATE.selected_index.store(-1, Ordering::Relaxed);
            return;
        } else if rx >= 74 && rx <= 98 {
            // View Mode Toggle Button (List vs Grid)
            FILE_MANAGER_STATE.toggle_view_mode();
            return;
        } else if rx >= win_width as i32 - 134 && rx <= win_width as i32 - 88 {
            // +Dir Button
            FILE_MANAGER_STATE.open_modal(1);
            return;
        } else if rx >= win_width as i32 - 84 && rx <= win_width as i32 - 42 {
            // +File Button
            FILE_MANAGER_STATE.open_modal(2);
            return;
        } else if rx >= win_width as i32 - 38 && rx <= win_width as i32 - 14 {
            // Delete Button (X)
            if FILE_MANAGER_STATE.selected_index.load(Ordering::Relaxed) >= 0 {
                FILE_MANAGER_STATE.open_modal(3);
            }
            return;
        }
    }

    // Layout Dimensions for Directory Item List Area
    let list_width = (win_width as i32 - 170).max(180);
    let view_mode = FILE_MANAGER_STATE.view_mode.load(Ordering::Relaxed);
    let scroll_offset = FILE_MANAGER_STATE.scroll_offset.load(Ordering::Relaxed);
    let container_h = win_height as i32 - 106;

    // Check Scrollbar & Scroll Button Clicks (rx near list_width - 16 to list_width + 10)
    if rx >= list_width - 16 && rx <= list_width + 10 && ry >= 76 && ry < win_height as i32 - 30 {
        if ry >= 76 && ry <= 92 {
            // Up Scroll Button (^)
            FILE_MANAGER_STATE.scroll_up(28);
            return;
        } else if ry >= win_height as i32 - 46 && ry <= win_height as i32 - 30 {
            // Down Scroll Button (v)
            FILE_MANAGER_STATE.scroll_down(28, container_h, list_width);
            return;
        } else {
            // Track / Thumb Click
            let max_scroll = FILE_MANAGER_STATE.calculate_max_scroll(container_h, list_width);
            if max_scroll > 0 {
                let track_rel_y = ry - 94;
                let track_h = (container_h - 36).max(1);
                let target = ((track_rel_y as f32 / track_h as f32) * max_scroll as f32) as i32;
                FILE_MANAGER_STATE.scroll_offset.store(target.min(max_scroll).max(0), Ordering::Relaxed);
            }
            return;
        }
    }

    // Check Directory Item List Clicks (ry >= 76)
    if ry >= 76 && rx >= 12 && rx < list_width - 16 {
        let mut clicked_idx = -1i32;

        if view_mode == 0 {
            // List view clicked row
            clicked_idx = (ry - 76 + scroll_offset) / 28;
        } else {
            // Grid view clicked card
            let card_w = 64i32;
            let card_h = 60i32;
            let gap_x = 10i32;
            let gap_y = 10i32;
            let cols = ((list_width - 24) / (card_w + gap_x)).max(1);
            let c = (rx - 16) / (card_w + gap_x);
            let r = (ry - 76 + scroll_offset) / (card_h + gap_y);
            if c >= 0 && c < cols {
                clicked_idx = r * cols + c;
            }
        }

        if clicked_idx < 0 {
            return;
        }

        let mut path_tmp = [0u8; 256];
        let current_path = FILE_MANAGER_STATE.get_path(&mut path_tmp);

        let fd = sys_open(current_path.as_ptr(), current_path.len(), 0);
        if fd != u64::MAX && fd < 16 {
            let mut dir_buf = [0u8; 1024];
            let bytes_read = sys_read(fd, dir_buf.as_mut_ptr(), 1024);
            sys_close(fd);
            if bytes_read != u64::MAX && bytes_read > 0 {
                let slice = &dir_buf[..bytes_read as usize];
                if let Ok(s) = core::str::from_utf8(slice) {
                    for (idx, entry) in s.lines().enumerate() {
                        if idx as i32 == clicked_idx {
                            let prev_selected = FILE_MANAGER_STATE.selected_index.load(Ordering::Relaxed);
                            if prev_selected == idx as i32 && entry.ends_with('/') {
                                // Repeat click on already selected directory enters it
                                FILE_MANAGER_STATE.push_dir(entry);
                            } else {
                                // Select item for preview
                                FILE_MANAGER_STATE.selected_index.store(idx as i32, Ordering::Relaxed);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}
