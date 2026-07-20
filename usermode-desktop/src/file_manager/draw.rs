// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # File Manager Rendering Routines
//!
//! Provides visual rendering of toolbar controls, directory items (List & Grid modes),
//! side preview drawer, vertical scrollbars, bottom status bar, and modal overlay pop-ups.

use core::sync::atomic::Ordering;
use crate::utils::StrbufWriter;
use crate::syscalls::{sys_open, sys_read, sys_close};
use crate::atlas_font::{draw_text_atlas, AtlasSize, AtlasWeight};
use crate::graphics::{draw_rect, draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha, draw_tiny_folder_icon, draw_tiny_file_icon};
use super::state::{FILE_MANAGER_STATE, detect_file_type};

/// Renders the visual content of the File Manager application window.
///
/// # Arguments
///
/// * `ax` - Animated starting X-coordinate of the window body.
/// * `ay` - Animated starting Y-coordinate of the window body.
/// * `win_width` - Active width of the window.
/// * `win_height` - Active height of the window.
/// * `cursor_x` - Active screen X-coordinate of the mouse cursor.
/// * `cursor_y` - Active screen Y-coordinate of the mouse cursor.
pub fn draw_file_manager(
    ax: i32,
    ay: i32,
    win_width: usize,
    win_height: usize,
    cursor_x: i32,
    cursor_y: i32,
) {
    let mut path_tmp = [0u8; 256];
    let path_str = FILE_MANAGER_STATE.get_path(&mut path_tmp);
    let view_mode = FILE_MANAGER_STATE.view_mode.load(Ordering::Relaxed);

    // ────────────────────────────────────────────────────────
    // 1. Navigation & Operations Toolbar
    // ────────────────────────────────────────────────────────
    draw_rounded_rect_alpha(ax + 12, ay + 38, win_width as i32 - 24, 30, 6, 30, 34, 46, 220);

    let rx = cursor_x - ax;
    let ry = cursor_y - ay;

    // Back Button (<)
    let back_hovered = ry >= 38 && ry <= 68 && rx >= 14 && rx <= 30;
    if back_hovered {
        draw_rounded_rect_alpha(ax + 14, ay + 41, 16, 24, 4, 255, 255, 255, 30);
    }
    draw_text_atlas(ax + 19, ay + 45, "<", 220, 230, 245, AtlasSize::Small, AtlasWeight::SemiBold);

    // Up / Root Button (^)
    let up_hovered = ry >= 38 && ry <= 68 && rx >= 34 && rx <= 50;
    if up_hovered {
        draw_rounded_rect_alpha(ax + 34, ay + 41, 16, 24, 4, 255, 255, 255, 30);
    }
    draw_text_atlas(ax + 39, ay + 45, "^", 220, 230, 245, AtlasSize::Small, AtlasWeight::SemiBold);

    // Refresh Button (R)
    let refresh_hovered = ry >= 38 && ry <= 68 && rx >= 54 && rx <= 70;
    if refresh_hovered {
        draw_rounded_rect_alpha(ax + 54, ay + 41, 16, 24, 4, 255, 255, 255, 30);
    }
    draw_text_atlas(ax + 59, ay + 45, "R", 220, 230, 245, AtlasSize::Small, AtlasWeight::SemiBold);

    // View Mode Toggle Button ([Lst] / [Grd])
    let view_btn_hovered = ry >= 38 && ry <= 68 && rx >= 74 && rx <= 98;
    let view_bg_alpha = if view_btn_hovered { 40 } else { 15 };
    draw_rounded_rect_alpha(ax + 74, ay + 41, 24, 24, 4, 255, 255, 255, view_bg_alpha);
    let view_label = if view_mode == 0 { "Lst" } else { "Grd" };
    draw_text_atlas(ax + 77, ay + 45, view_label, 200, 220, 245, AtlasSize::Small, AtlasWeight::SemiBold);

    // Vertical Divider
    draw_rect(ax + 104, ay + 43, 1, 20, 60, 65, 80);

    // Action Buttons (Right-aligned in Toolbar)
    // +Folder Button
    let btn_dir_x = ax + win_width as i32 - 134;
    let dir_btn_hovered = ry >= 38 && ry <= 68 && rx >= win_width as i32 - 134 && rx <= win_width as i32 - 88;
    let dir_alpha = if dir_btn_hovered { 230 } else { 170 };
    draw_rounded_rect_alpha(btn_dir_x, ay + 41, 46, 24, 4, 61, 174, 233, dir_alpha);
    draw_text_atlas(btn_dir_x + 6, ay + 45, "+Dir", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

    // +File Button
    let btn_file_x = ax + win_width as i32 - 84;
    let file_btn_hovered = ry >= 38 && ry <= 68 && rx >= win_width as i32 - 84 && rx <= win_width as i32 - 42;
    let file_alpha = if file_btn_hovered { 230 } else { 170 };
    draw_rounded_rect_alpha(btn_file_x, ay + 41, 42, 24, 4, 61, 174, 233, file_alpha);
    draw_text_atlas(btn_file_x + 5, ay + 45, "+File", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

    // Delete Button (X)
    let btn_del_x = ax + win_width as i32 - 38;
    let del_btn_hovered = ry >= 38 && ry <= 68 && rx >= win_width as i32 - 38 && rx <= win_width as i32 - 14;
    let del_alpha = if del_btn_hovered { 230 } else { 170 };
    draw_rounded_rect_alpha(btn_del_x, ay + 41, 24, 24, 4, 220, 70, 70, del_alpha);
    draw_text_atlas(btn_del_x + 8, ay + 45, "X", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

    // Active Path Display (Breadcrumbs - Truncated if long)
    let path_display = if path_str.len() > 14 { &path_str[..14] } else { path_str };
    let mut path_fmt_buf = [0u8; 128];
    let mut path_writer = StrbufWriter::new(&mut path_fmt_buf);
    let _ = core::fmt::write(&mut path_writer, format_args!("Path: {}", path_display));
    draw_text_atlas(ax + 112, ay + 45, path_writer.as_str(), 180, 205, 235, AtlasSize::Small, AtlasWeight::SemiBold);

    // Horizontal Separator Line
    draw_rect(ax + 10, ay + 72, win_width as i32 - 20, 1, 48, 52, 70);

    // Layout Dimensions for Split View
    let list_width = (win_width as i32 - 170).max(180);
    let preview_x = ax + list_width + 18;
    let preview_width = (win_width as i32 - (list_width + 30)).max(120);

    // Vertical Separator Line between Directory List and Preview Drawer
    draw_rect(ax + list_width + 14, ay + 72, 1, win_height as i32 - 100, 48, 52, 70);

    // ────────────────────────────────────────────────────────
    // 2. Directory Items Listing (Left Section - List or Grid)
    // ────────────────────────────────────────────────────────
    let selected_idx = FILE_MANAGER_STATE.selected_index.load(Ordering::Relaxed);
    let scroll_offset = FILE_MANAGER_STATE.scroll_offset.load(Ordering::Relaxed);

    let mut selected_entry_name = [0u8; 64];
    let mut selected_entry_len = 0usize;
    let mut is_selected_entry_dir = false;
    let mut total_items = 0usize;

    let fd = sys_open(path_str.as_ptr(), path_str.len(), 0);
    if fd != u64::MAX && fd < 16 {
        let mut dir_buf = [0u8; 1024];
        let bytes_read = sys_read(fd, dir_buf.as_mut_ptr(), 1024);
        sys_close(fd);
        if bytes_read != u64::MAX && bytes_read > 0 {
            let slice = &dir_buf[..bytes_read as usize];
            if let Ok(s) = core::str::from_utf8(slice) {
                total_items = s.lines().count();
                FILE_MANAGER_STATE.total_items.store(total_items as u32, Ordering::Relaxed);

                if view_mode == 0 {
                    // ── LIST VIEW MODE ──
                    let in_item_area = rx >= 12 && rx < list_width - 16 && ry >= 76 && ry < win_height as i32 - 30;
                    let hovered_row = if in_item_area { (ry - 76 + scroll_offset) / 28 } else { -1 };
                    FILE_MANAGER_STATE.hovered_index.store(hovered_row, Ordering::Relaxed);

                    let mut line_y = ay + 76 - scroll_offset;
                    for (idx, entry) in s.lines().enumerate() {
                        let is_selected = selected_idx == idx as i32;
                        let is_hovered = hovered_row == idx as i32;

                        if is_selected {
                            let name_bytes = entry.as_bytes();
                            let copy_len = core::cmp::min(name_bytes.len(), 64);
                            selected_entry_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
                            selected_entry_len = copy_len;
                            is_selected_entry_dir = entry.ends_with('/');
                        }

                        if line_y >= ay + 76 && line_y + 26 <= ay + win_height as i32 - 30 {
                            if is_selected {
                                draw_rounded_rect_alpha(ax + 12, line_y, list_width - 26, 26, 6, 61, 174, 233, 200);
                            } else if is_hovered {
                                draw_rounded_rect_alpha(ax + 12, line_y, list_width - 26, 26, 6, 255, 255, 255, 25);
                            }

                            let mut entry_buf = [0u8; 64];
                            let mut writer = StrbufWriter::new(&mut entry_buf);
                            
                            if entry.ends_with('/') {
                                draw_tiny_folder_icon(ax + 20, line_y + 7);
                                let display_name = &entry[..entry.len() - 1];
                                let trunc_name = if display_name.len() > 14 { &display_name[..14] } else { display_name };
                                let _ = core::fmt::write(&mut writer, format_args!("{}", trunc_name));
                                let (tr, tg, tb) = if is_selected { (255, 255, 255) } else { (200, 215, 240) };
                                draw_text_atlas(ax + 44, line_y + 5, writer.as_str(), tr, tg, tb, AtlasSize::Small, AtlasWeight::SemiBold);
                            } else {
                                draw_tiny_file_icon(ax + 20, line_y + 6);
                                let trunc_name = if entry.len() > 14 { &entry[..14] } else { entry };
                                let _ = core::fmt::write(&mut writer, format_args!("{}", trunc_name));
                                let (tr, tg, tb) = if is_selected { (255, 255, 255) } else { (215, 222, 238) };
                                draw_text_atlas(ax + 44, line_y + 5, writer.as_str(), tr, tg, tb, AtlasSize::Small, AtlasWeight::Regular);
                            }
                        }

                        line_y += 28;
                    }
                } else {
                    // ── GRID VIEW MODE (CARDS) ──
                    let card_w = 64i32;
                    let card_h = 60i32;
                    let gap_x = 10i32;
                    let gap_y = 10i32;
                    let cols = ((list_width - 24) / (card_w + gap_x)).max(1);

                    let in_item_area = rx >= 12 && rx < list_width - 16 && ry >= 76 && ry < win_height as i32 - 30;
                    let hovered_idx = if in_item_area {
                        let c = (rx - 16) / (card_w + gap_x);
                        let r = (ry - 76 + scroll_offset) / (card_h + gap_y);
                        if c >= 0 && c < cols { r * cols + c } else { -1 }
                    } else {
                        -1
                    };
                    FILE_MANAGER_STATE.hovered_index.store(hovered_idx, Ordering::Relaxed);

                    for (idx, entry) in s.lines().enumerate() {
                        let col = (idx as i32) % cols;
                        let row = (idx as i32) / cols;

                        let cx = ax + 16 + col * (card_w + gap_x);
                        let cy = ay + 78 + row * (card_h + gap_y) - scroll_offset;

                        let is_selected = selected_idx == idx as i32;
                        let is_hovered = hovered_idx == idx as i32;

                        if is_selected {
                            let name_bytes = entry.as_bytes();
                            let copy_len = core::cmp::min(name_bytes.len(), 64);
                            selected_entry_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
                            selected_entry_len = copy_len;
                            is_selected_entry_dir = entry.ends_with('/');
                        }

                        if cy >= ay + 76 && cy + card_h <= ay + win_height as i32 - 30 {
                            if is_selected {
                                draw_rounded_rect_alpha(cx, cy, card_w, card_h, 8, 61, 174, 233, 200);
                            } else if is_hovered {
                                draw_rounded_rect_alpha(cx, cy, card_w, card_h, 8, 255, 255, 255, 30);
                            } else {
                                draw_rounded_rect_alpha(cx, cy, card_w, card_h, 8, 30, 35, 48, 160);
                            }

                            let display_name = entry.trim_end_matches('/');
                            let mut entry_buf = [0u8; 64];
                            let mut writer = StrbufWriter::new(&mut entry_buf);
                            let _ = core::fmt::write(&mut writer, format_args!("{}", display_name));
                            let name_str = writer.as_str();
                            let truncated = if name_str.len() > 7 { &name_str[..7] } else { name_str };

                            if entry.ends_with('/') {
                                draw_tiny_folder_icon(cx + 24, cy + 10);
                                draw_text_atlas(cx + 6, cy + 36, truncated, 240, 245, 255, AtlasSize::Small, AtlasWeight::SemiBold);
                            } else {
                                draw_tiny_file_icon(cx + 26, cy + 10);
                                draw_text_atlas(cx + 6, cy + 36, truncated, 220, 230, 245, AtlasSize::Small, AtlasWeight::Regular);
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Vertical Scrollbar & Scroll Track ──
    let container_h = win_height as i32 - 106;
    let max_scroll = FILE_MANAGER_STATE.calculate_max_scroll(container_h, list_width);

    if max_scroll > 0 {
        let sb_x = ax + list_width - 12;

        // Up Arrow Scroll Button (^)
        let up_arrow_hovered = rx >= list_width - 14 && rx <= list_width && ry >= 76 && ry <= 92;
        let up_alpha = if up_arrow_hovered { 230 } else { 120 };
        draw_rounded_rect_alpha(sb_x, ay + 76, 12, 16, 3, 61, 174, 233, up_alpha);
        draw_text_atlas(sb_x + 3, ay + 77, "^", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

        // Scroll Track
        let track_y = ay + 94;
        let track_h = container_h - 36;
        draw_rounded_rect_alpha(sb_x + 2, track_y, 8, track_h, 4, 255, 255, 255, 25);

        // Scroll Thumb
        let thumb_h = ((track_h as f32 * 0.35) as i32).max(16);
        let thumb_y = track_y + ((track_h - thumb_h) as f32 * (scroll_offset as f32 / max_scroll as f32)) as i32;
        draw_rounded_rect_alpha(sb_x + 2, thumb_y, 8, thumb_h, 4, 61, 174, 233, 230);

        // Down Arrow Scroll Button (v)
        let down_y = ay + win_height as i32 - 46;
        let down_arrow_hovered = rx >= list_width - 14 && rx <= list_width && ry >= win_height as i32 - 46 && ry <= win_height as i32 - 30;
        let down_alpha = if down_arrow_hovered { 230 } else { 120 };
        draw_rounded_rect_alpha(sb_x, down_y, 12, 16, 3, 61, 174, 233, down_alpha);
        draw_text_atlas(sb_x + 3, down_y + 1, "v", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);
    }

    // ────────────────────────────────────────────────────────
    // 3. Side Quick Preview Drawer (Right Section)
    // ────────────────────────────────────────────────────────
    draw_text_atlas(preview_x, ay + 78, "PREVIEW", 130, 145, 175, AtlasSize::Small, AtlasWeight::SemiBold);

    if selected_entry_len == 0 {
        // Empty State: No Item Selected
        draw_rounded_rect_alpha(preview_x, ay + 98, preview_width, win_height as i32 - 130, 6, 255, 255, 255, 10);
        draw_text_atlas(preview_x + 8, ay + 114, "No item selected", 140, 150, 170, AtlasSize::Small, AtlasWeight::Regular);
        draw_text_atlas(preview_x + 8, ay + 132, "Click a file", 110, 120, 140, AtlasSize::Small, AtlasWeight::Regular);
        draw_text_atlas(preview_x + 8, ay + 148, "to inspect.", 110, 120, 140, AtlasSize::Small, AtlasWeight::Regular);
    } else {
        if let Ok(entry_str) = core::str::from_utf8(&selected_entry_name[..selected_entry_len]) {
            let type_str = detect_file_type(entry_str);
            let display_name = entry_str.trim_end_matches('/');
            let name_trunc = if display_name.len() > 10 { &display_name[..10] } else { display_name };

            // Render Item Icon & Name
            if is_selected_entry_dir {
                draw_tiny_folder_icon(preview_x, ay + 100);
            } else {
                draw_tiny_file_icon(preview_x, ay + 99);
            }
            draw_text_atlas(preview_x + 22, ay + 98, name_trunc, 230, 240, 255, AtlasSize::Small, AtlasWeight::SemiBold);

            // Render Shortened Item Type Badge
            let type_short = match type_str {
                "Folder" => "Folder",
                "Rust Source Code" => "Rust Source",
                "Text Document" => "Text Doc",
                "System Log File" => "System Log",
                _ => "File",
            };
            let mut type_buf = [0u8; 64];
            let mut type_writer = StrbufWriter::new(&mut type_buf);
            let _ = core::fmt::write(&mut type_writer, format_args!("Type: {}", type_short));
            draw_text_atlas(preview_x, ay + 120, type_writer.as_str(), 160, 175, 205, AtlasSize::Small, AtlasWeight::Regular);

            // Construct Full System Path for Inspection
            let mut full_path_buf = [0u8; 256];
            let mut path_writer = StrbufWriter::new(&mut full_path_buf);
            if path_str == "/" {
                let _ = core::fmt::write(&mut path_writer, format_args!("/{}", entry_str));
            } else {
                let _ = core::fmt::write(&mut path_writer, format_args!("{}/{}", path_str, entry_str));
            }
            let full_path = path_writer.as_str();

            if is_selected_entry_dir {
                // Folder Inspection Info
                draw_text_atlas(preview_x, ay + 140, "Folder Node", 130, 190, 230, AtlasSize::Small, AtlasWeight::Regular);
                draw_rounded_rect_alpha(preview_x, ay + 162, preview_width, win_height as i32 - 192, 6, 20, 24, 34, 220);
                draw_text_atlas(preview_x + 6, ay + 172, "Click again to", 140, 155, 180, AtlasSize::Small, AtlasWeight::Regular);
                draw_text_atlas(preview_x + 6, ay + 190, "enter dir.", 140, 155, 180, AtlasSize::Small, AtlasWeight::Regular);
            } else {
                // Open File & Inspect Content
                let file_fd = sys_open(full_path.as_ptr(), full_path.len(), 0);
                if file_fd != u64::MAX && file_fd < 16 {
                    let mut file_data = [0u8; 512];
                    let file_read = sys_read(file_fd, file_data.as_mut_ptr(), 512);
                    sys_close(file_fd);

                    if file_read != u64::MAX {
                        // Display Size
                        let mut size_buf = [0u8; 64];
                        let mut size_writer = StrbufWriter::new(&mut size_buf);
                        let _ = core::fmt::write(&mut size_writer, format_args!("Size: {} B", file_read));
                        draw_text_atlas(preview_x, ay + 140, size_writer.as_str(), 140, 185, 230, AtlasSize::Small, AtlasWeight::Regular);

                        // Live Multi-Line Text Preview Box
                        draw_rounded_rect_alpha(preview_x, ay + 162, preview_width, win_height as i32 - 192, 6, 20, 24, 34, 220);
                        draw_text_atlas(preview_x + 6, ay + 168, "File Preview:", 120, 140, 170, AtlasSize::Small, AtlasWeight::SemiBold);

                        let preview_bytes = &file_data[..file_read as usize];
                        if let Ok(preview_str) = core::str::from_utf8(preview_bytes) {
                            let mut text_y = ay + 188;
                            for line in preview_str.lines().take(5) {
                                if text_y + 14 > ay + win_height as i32 - 32 {
                                    break;
                                }
                                let display_line = if line.len() > 12 { &line[..12] } else { line };
                                draw_text_atlas(preview_x + 6, text_y, display_line, 195, 205, 225, AtlasSize::Small, AtlasWeight::Regular);
                                text_y += 16;
                            }
                        }
                    }
                }
            }
        }
    }

    // ────────────────────────────────────────────────────────
    // 4. Phase 5: Bottom Status Bar (Items & Disk Storage Info)
    // ────────────────────────────────────────────────────────
    let status_y = ay + win_height as i32 - 24;
    draw_rounded_rect_alpha(ax + 10, status_y, win_width as i32 - 20, 18, 4, 22, 26, 36, 235);

    let mut status_buf = [0u8; 128];
    let mut status_writer = StrbufWriter::new(&mut status_buf);
    if selected_entry_len > 0 {
        if let Ok(name) = core::str::from_utf8(&selected_entry_name[..selected_entry_len]) {
            let clean = name.trim_end_matches('/');
            let clean_trunc = if clean.len() > 10 { &clean[..10] } else { clean };
            let _ = core::fmt::write(&mut status_writer, format_args!("{} Items | Sel: {}", total_items, clean_trunc));
        } else {
            let _ = core::fmt::write(&mut status_writer, format_args!("{} Items", total_items));
        }
    } else {
        let _ = core::fmt::write(&mut status_writer, format_args!("{} Items", total_items));
    }
    draw_text_atlas(ax + 18, status_y + 3, status_writer.as_str(), 170, 185, 210, AtlasSize::Small, AtlasWeight::Regular);

    let storage_str = "VFS: OK";
    draw_text_atlas(ax + win_width as i32 - 65, status_y + 3, storage_str, 120, 180, 220, AtlasSize::Small, AtlasWeight::Regular);

    // ────────────────────────────────────────────────────────
    // 5. Modal Dialog Overlay (Creation / Deletion Dialogs)
    // ────────────────────────────────────────────────────────
    let modal_mode = FILE_MANAGER_STATE.modal_mode.load(Ordering::Relaxed);
    if modal_mode > 0 {
        let mw = 280i32;
        let mh = 140i32;
        let mx = ax + (win_width as i32 - mw) / 2;
        let my = ay + (win_height as i32 - mh) / 2;

        // Dark Backdrop Overlay
        draw_rounded_rect_alpha(ax + 10, ay + 34, win_width as i32 - 20, win_height as i32 - 44, 10, 10, 12, 18, 160);

        // Modal Glass Box
        draw_rounded_rect_alpha(mx, my, mw, mh, 10, 24, 28, 38, 245);
        draw_rounded_rect_outline_alpha(mx, my, mw, mh, 10, 61, 174, 233, 1, 220);

        let modal_title = match modal_mode {
            1 => "Create New Folder",
            2 => "Create New File",
            3 => "Delete Selected Item?",
            _ => "Dialog",
        };
        draw_text_atlas(mx + 16, my + 14, modal_title, 240, 248, 255, AtlasSize::Small, AtlasWeight::SemiBold);

        if modal_mode == 1 || modal_mode == 2 {
            // Text Input Box
            draw_rounded_rect_alpha(mx + 16, my + 44, 248, 28, 6, 12, 14, 20, 255);
            draw_rounded_rect_outline_alpha(mx + 16, my + 44, 248, 28, 6, 80, 100, 130, 1, 150);

            let mut input_tmp = [0u8; 64];
            let typed_str = FILE_MANAGER_STATE.get_modal_input(&mut input_tmp);
            if typed_str.is_empty() {
                let placeholder = if modal_mode == 1 { "Enter folder name..." } else { "Enter file name..." };
                draw_text_atlas(mx + 24, my + 50, placeholder, 110, 120, 140, AtlasSize::Small, AtlasWeight::Regular);
            } else {
                draw_text_atlas(mx + 24, my + 50, typed_str, 255, 255, 255, AtlasSize::Small, AtlasWeight::Regular);
            }
        } else if modal_mode == 3 {
            draw_text_atlas(mx + 16, my + 50, "Are you sure you want to proceed?", 190, 200, 220, AtlasSize::Small, AtlasWeight::Regular);
        }

        // Cancel Button
        let cancel_hovered = ry >= my - ay + 92 && ry <= my - ay + 120 && rx >= mx - ax + 104 && rx <= mx - ax + 174;
        let cancel_alpha = if cancel_hovered { 255 } else { 220 };
        draw_rounded_rect_alpha(mx + 104, my + 92, 70, 28, 6, 60, 65, 80, cancel_alpha);
        draw_text_atlas(mx + 118, my + 98, "Cancel", 220, 225, 235, AtlasSize::Small, AtlasWeight::Regular);

        // Action/OK Button
        let ok_hovered = ry >= my - ay + 92 && ry <= my - ay + 120 && rx >= mx - ax + 184 && rx <= mx - ax + 264;
        let ok_alpha = if ok_hovered { 255 } else { 220 };
        draw_rounded_rect_alpha(mx + 184, my + 92, 80, 28, 6, 61, 174, 233, ok_alpha);
        let btn_label = if modal_mode == 3 { "Confirm" } else { "Create" };
        draw_text_atlas(mx + 200, my + 98, btn_label, 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);
    }
}
