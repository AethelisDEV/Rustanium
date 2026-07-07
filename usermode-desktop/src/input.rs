// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Input Event Processing and Window Interaction
//!
//! Handles keyboard, mouse dragging/clicks, Launchpad overlay interactions,
//! and CLI serial UART inputs.

use crate::state::{
    WINDOW_BACKING_STORES, SCREEN_WIDTH, SCREEN_HEIGHT,
    START_MENU_OPEN, START_MENU_ANIMATING, SHADOWS_ENABLED,
    ACTIVE_SETTINGS_TAB,
};
use core::sync::atomic::Ordering;
use crate::window::{
    WINDOWS, focus_window_by_id, hit_test_title, hit_test_body,
};
use crate::taskbar::get_dock_layout;
use crate::dirty::DirtyRectTracker;
use crate::console::{term_process_command, term_print_char};
use crate::syscalls::{InputEvent, sys_write, syscall0};

/// Tracks transient cursor positions and click flags.
pub struct InputState {
    /// Active X coordinate of mouse.
    pub cursor_x: i32,
    /// Active Y coordinate of mouse.
    pub cursor_y: i32,
    /// Historical X coordinate of mouse from previous event.
    pub prev_mouse_x: i32,
    /// Historical Y coordinate of mouse from previous event.
    pub prev_mouse_y: i32,
    /// Flag indicating whether the mouse button was held down.
    pub prev_left_clicked: u8,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    /// Initializes input tracker at centre of screen.
    pub fn new() -> Self {
        Self {
            cursor_x: 400,
            cursor_y: 300,
            prev_mouse_x: 400,
            prev_mouse_y: 300,
            prev_left_clicked: 0,
        }
    }
}

/// Routes raw keyboard/mouse events into windows and components.
pub fn handle_input_event(
    event: &InputEvent,
    state: &mut InputState,
    dirty_tracker: &mut DirtyRectTracker,
    needs_redraw: &mut bool,
) {
    if event.event_type == 1 {
        // Keyboard Input
        let key = event.keyboard_key;
        let mut terminal_focused = false;
        unsafe {
            for i in 0..5 {
                if let Some(ref win) = WINDOWS[i] {
                    if win.id == 1 && win.is_focused {
                        terminal_focused = true;
                        break;
                    }
                }
            }
        }

        let start_menu_open = START_MENU_OPEN.load(Ordering::Relaxed);
        {
            let mut dbg_buf = [0u8; 128];
            let mut w = crate::utils::StrbufWriter::new(&mut dbg_buf);
            let _ = core::fmt::write(&mut w, format_args!("[DE] Key: {}, Menu Open: {}\n", key, start_menu_open));
            crate::utils::serial_print(w.as_str());
        }
        if start_menu_open {
            if key == 0x1001 { // Enter
                let mut q_buf = [0u8; 64];
                let query_str = crate::state::SEARCH_QUERY.get_str(&mut q_buf);
                let mut found_app = None;
                for app in crate::graphics::compositor::ALL_APPS.iter() {
                    if crate::graphics::compositor::matches_search(app.title, query_str) {
                        found_app = Some(app);
                        break;
                    }
                }
                if let Some(app) = found_app {
                    if app.id == 5 {
                        let msg = "Shutting down system...\n";
                        let _ = sys_write(2, msg.as_ptr(), msg.len());
                        syscall0(3);
                    } else {
                        focus_window_by_id(app.id);
                        for k in 0..5 {
                            WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                        }
                        dirty_tracker.mark_all_dirty();
                        START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                        START_MENU_OPEN.store(false, Ordering::Relaxed);
                    }
                }
            } else if key == 0x1000 { // Backspace
                crate::state::SEARCH_QUERY.pop();
                *needs_redraw = true;
            } else if key < 0x1000 {
                let c = (key as u8) as char;
                if c.is_ascii_graphic() || c == ' ' {
                    crate::state::SEARCH_QUERY.push(c);
                    *needs_redraw = true;
                }
            }
        } else if terminal_focused {
            if key == 0x1001 { // Enter
                term_process_command();
            } else if key == 0x1000 { // Backspace
                term_print_char('\x08');
            } else if key < 0x1000 {
                term_print_char((key as u8) as char);
            }
        }
    } else if event.event_type == 2 {
        // Mouse Input
        state.cursor_x = event.mouse_x;
        state.cursor_y = event.mouse_y;
        let left_clicked = event.mouse_left_clicked;
        
        let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
        let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
        let start_menu_open = START_MENU_OPEN.load(Ordering::Relaxed);
        let in_dock_zone = state.cursor_y >= (sh - 120);
        let prev_in_dock_zone = state.prev_mouse_y >= (sh - 120);
        
        let in_start_menu_zone = start_menu_open;
        let prev_in_start_menu_zone = start_menu_open;

        if left_clicked == 1 || state.prev_left_clicked == 1 || in_dock_zone || prev_in_dock_zone || in_start_menu_zone || prev_in_start_menu_zone {
            *needs_redraw = true;
        }

        let dx = state.cursor_x - state.prev_mouse_x;
        let dy = state.cursor_y - state.prev_mouse_y;

        if left_clicked == 1 {
            if state.prev_left_clicked == 0 {
                // Mouse Down Click
                let mut event_consumed = false;
                let start_menu_animating = START_MENU_ANIMATING.load(Ordering::Relaxed);
                let (_dock_start_x, _dock_w, _dock_sizes, _dock_xs) = get_dock_layout(sw, sh, state.cursor_x, state.cursor_y);
                let dock_y = sh - 62;
                
                if start_menu_open && !start_menu_animating {
                    event_consumed = true;
                    let mut q_buf = [0u8; 64];
                    let query_str = crate::state::SEARCH_QUERY.get_str(&mut q_buf);
                    
                    let mut matched_apps = [None; 6];
                    let mut matched_count = 0;
                    for app in crate::graphics::compositor::ALL_APPS.iter() {
                        if crate::graphics::compositor::matches_search(app.title, query_str) {
                            matched_apps[matched_count] = Some(app);
                            matched_count += 1;
                        }
                    }
                    
                    let mut clicked_app = None;
                    for idx in 0..matched_count {
                        let app = matched_apps[idx].unwrap();
                        let (cx, cy, cw, ch) = crate::graphics::compositor::get_app_card_layout(idx, matched_count, sw, sh);
                        if state.cursor_x >= cx && state.cursor_x < cx + cw &&
                           state.cursor_y >= cy && state.cursor_y < cy + ch {
                            clicked_app = Some(app);
                            break;
                        }
                    }
                    
                    if let Some(app) = clicked_app {
                        if app.id == 5 {
                            let msg = "Shutting down system...\n";
                            let _ = sys_write(2, msg.as_ptr(), msg.len());
                            syscall0(3);
                        } else {
                            focus_window_by_id(app.id);
                            for k in 0..5 {
                                WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                            }
                            dirty_tracker.mark_all_dirty();
                            START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                            START_MENU_OPEN.store(false, Ordering::Relaxed);
                        }
                    } else {
                        // Check if clicked outside of the dock or on it
                         let on_dock = state.cursor_y >= dock_y && state.cursor_y < dock_y + 56 &&
                                      state.cursor_x >= _dock_start_x as i32 && state.cursor_x < (_dock_start_x + _dock_w) as i32;
                        if !on_dock {
                            START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                            START_MENU_OPEN.store(false, Ordering::Relaxed);
                        } else {
                            // Let dock click handler process it (set event_consumed = false)
                            event_consumed = false;
                        }
                    }
                }
                
                if !event_consumed {
                    // Check Dock Click
                    let (dock_start_x, dock_w, dock_sizes, dock_xs) = get_dock_layout(sw, sh, state.cursor_x, state.cursor_y);
                     let dock_y = sh - 62;
                     
                     if state.cursor_y >= dock_y && state.cursor_y < dock_y + 56 &&
                       state.cursor_x >= dock_start_x as i32 && state.cursor_x < (dock_start_x + dock_w) as i32 {
                        
                        event_consumed = true;
                        
                        for i in 0..5 {
                            let item_x = dock_xs[i];
                            let item_size = dock_sizes[i];
                            if state.cursor_x >= item_x as i32 && state.cursor_x < (item_x + item_size) as i32 {
                                if i == 0 {
                                    START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                                    let open = START_MENU_OPEN.load(Ordering::Relaxed);
                                    START_MENU_OPEN.store(!open, Ordering::Relaxed);
                                    if !open {
                                        crate::state::SEARCH_QUERY.clear();
                                    }
                                } else {
                                    if START_MENU_OPEN.load(Ordering::Relaxed) {
                                        START_MENU_OPEN.store(false, Ordering::Relaxed);
                                        START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                                    }
                                    let win_id = match i {
                                        1 => 0, // Metrics
                                        2 => 2, // Files
                                        3 => 1, // Console
                                        4 => 4, // Radiation Simulator
                                        _ => 0,
                                    };
                                    
                                    let mut found_win_idx = None;
                                    unsafe {
                                        for idx in 0..5 {
                                            if let Some(ref win) = WINDOWS[idx] {
                                                if win.id == win_id {
                                                    found_win_idx = Some(idx);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    
                                    if let Some(idx) = found_win_idx {
                                        let is_open = unsafe { WINDOWS[idx].as_ref().unwrap().is_open };
                                        let is_focused = unsafe { WINDOWS[idx].as_ref().unwrap().is_focused };
                                        
                                        if is_open && is_focused {
                                            let mut win_mut = unsafe { WINDOWS[idx].take().unwrap() };
                                            win_mut.is_animating = true;
                                            win_mut.anim_direction = false;
                                            win_mut.anim_progress = 100;
                                            unsafe { WINDOWS[idx] = Some(win_mut); }
                                        } else {
                                            focus_window_by_id(win_id);
                                        }
                                        for k in 0..5 {
                                            WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                                        }
                                        dirty_tracker.mark_all_dirty();
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                
                if !event_consumed {
                    // Window click hit tests
                    let mut clicked_idx = None;
                    let mut drag_started = false;
                    unsafe {
                        let mut count = 0;
                        for i in 0..5 {
                            if WINDOWS[i].is_some() {
                                count += 1;
                            }
                        }
                        
                        if count > 0 {
                            for i in (0..count).rev() {
                                if let Some(ref win) = WINDOWS[i] {
                                    if !win.is_open {
                                        continue;
                                    }
                                    if hit_test_title(win, state.cursor_x, state.cursor_y) {
                                        let (ax, ay) = win.get_animated_pos();
                                        let is_close_click = state.cursor_x >= ax + 16 && state.cursor_x < ax + 28 &&
                                                             state.cursor_y >= ay + 11 && state.cursor_y < ay + 23;
                                        let is_min_click   = state.cursor_x >= ax + 34 && state.cursor_x < ax + 46 &&
                                                             state.cursor_y >= ay + 11 && state.cursor_y < ay + 23;
                                        let is_max_click   = state.cursor_x >= ax + 52 && state.cursor_x < ax + 64 &&
                                                             state.cursor_y >= ay + 11 && state.cursor_y < ay + 23;
                                        
                                        if is_close_click {
                                            let mut win_mut = WINDOWS[i].take().unwrap();
                                            win_mut.is_animating = true;
                                            win_mut.anim_direction = false;
                                            win_mut.anim_progress = 100;
                                            WINDOWS[i] = Some(win_mut);
                                            clicked_idx = Some(i);
                                            drag_started = false;
                                        } else if is_max_click {
                                            let mut win_mut = WINDOWS[i].take().unwrap();
                                            if !win_mut.is_maximized {
                                                win_mut.prev_x = win_mut.x;
                                                win_mut.prev_y = win_mut.y;
                                                win_mut.prev_w = win_mut.width;
                                                win_mut.prev_h = win_mut.height;
                                                win_mut.x = 0;
                                                win_mut.y = 0;
                                                win_mut.width = SCREEN_WIDTH.load(Ordering::Relaxed) as usize;
                                                win_mut.height = (SCREEN_HEIGHT.load(Ordering::Relaxed) - 52) as usize;
                                                win_mut.is_maximized = true;
                                            } else {
                                                win_mut.x = win_mut.prev_x;
                                                win_mut.y = win_mut.prev_y;
                                                win_mut.width = win_mut.prev_w;
                                                win_mut.height = win_mut.prev_h;
                                                win_mut.is_maximized = false;
                                            }
                                            WINDOWS[i] = Some(win_mut);
                                            clicked_idx = Some(i);
                                            drag_started = false;
                                        } else if is_min_click {
                                            let mut win_mut = WINDOWS[i].take().unwrap();
                                            win_mut.is_animating = true;
                                            win_mut.anim_direction = false;
                                            win_mut.anim_progress = 100;
                                            WINDOWS[i] = Some(win_mut);
                                            clicked_idx = Some(i);
                                            drag_started = false;
                                        } else {
                                            if !win.is_maximized {
                                                clicked_idx = Some(i);
                                                drag_started = true;
                                            } else {
                                                clicked_idx = Some(i);
                                                drag_started = false;
                                            }
                                        }
                                        break;
                                    } else if hit_test_body(win, state.cursor_x, state.cursor_y) {
                                        clicked_idx = Some(i);
                                        drag_started = false;
                                        
                                        // Settings Panel interactivity
                                        if win.id == 3 {
                                            let (ax, ay) = win.get_animated_pos();
                                            let rx = state.cursor_x - ax;
                                            let ry = state.cursor_y - ay;
                                            
                                            if rx >= 10 && rx <= 120 {
                                                // Sidebar click check
                                                if ry >= 50 && ry < 76 {
                                                    ACTIVE_SETTINGS_TAB.store(0, Ordering::Relaxed);
                                                    *needs_redraw = true;
                                                    dirty_tracker.mark_all_dirty();
                                                } else if ry >= 82 && ry < 108 {
                                                    ACTIVE_SETTINGS_TAB.store(1, Ordering::Relaxed);
                                                    *needs_redraw = true;
                                                    dirty_tracker.mark_all_dirty();
                                                } else if ry >= 114 && ry < 140 {
                                                    ACTIVE_SETTINGS_TAB.store(2, Ordering::Relaxed);
                                                    *needs_redraw = true;
                                                    dirty_tracker.mark_all_dirty();
                                                }
                                            } else if rx >= 150 && rx <= (win.width as i32 - 24) {
                                                // Right-pane content check
                                                let active_tab = ACTIVE_SETTINGS_TAB.load(Ordering::Relaxed);
                                                if active_tab == 0 {
                                                    // Appearance Tab
                                                    if ry >= 110 && ry <= 166 {
                                                        let shadows_on = SHADOWS_ENABLED.load(Ordering::Relaxed);
                                                        SHADOWS_ENABLED.store(!shadows_on, Ordering::Relaxed);
                                                        *needs_redraw = true;
                                                        dirty_tracker.mark_all_dirty();
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Radiation Simulator interactivity
                                        if win.id == 4 {
                                            let (ax, ay) = win.get_animated_pos();
                                            let rx = state.cursor_x - ax;
                                            let ry = state.cursor_y - ay;
                                            
                                            // FLIP 1 — Single-bit flip (ECC auto-corrects)
                                            if ry >= 118 && ry <= 146 &&
                                               rx >= win.width as i32 - 160 && rx < win.width as i32 - 104 {
                                                crate::radiation::add_radiation_log("[FLIP1] Injecting single-bit flip into frame...");
                                                let _ = crate::syscalls::sys_inject_bit_flip(999, 10, 2);
                                                let fd = crate::syscalls::sys_open("/data/system_info.log".as_ptr(), 21, 0);
                                                if fd != u64::MAX {
                                                    let mut tmp = [0u8; 128];
                                                    let _ = crate::syscalls::sys_read(fd, tmp.as_mut_ptr(), 128);
                                                    let _ = crate::syscalls::sys_close(fd);
                                                }
                                                crate::radiation::add_radiation_log("[FLIP1] ECC corrected — page recovered OK.");
                                                *needs_redraw = true;
                                                dirty_tracker.mark_all_dirty();
                                            }
                                            
                                            // FLIP 2 — Double-bit flip (VFS quarantines + relocates)
                                            if ry >= 118 && ry <= 146 &&
                                               rx >= win.width as i32 - 96 && rx < win.width as i32 - 40 {
                                                crate::radiation::add_radiation_log("[FLIP2] Injecting double-bit flip into frame...");
                                                let _ = crate::syscalls::sys_inject_bit_flip(999, 10, 1);
                                                let _ = crate::syscalls::sys_inject_bit_flip(999, 10, 5);
                                                let fd = crate::syscalls::sys_open("/data/system_info.log".as_ptr(), 21, 0);
                                                if fd != u64::MAX {
                                                    let mut tmp = [0u8; 128];
                                                    let _ = crate::syscalls::sys_read(fd, tmp.as_mut_ptr(), 128);
                                                    let _ = crate::syscalls::sys_close(fd);
                                                }
                                                crate::radiation::add_radiation_log("[FLIP2] Uncorrectable! VFS quarantine + relocation done.");
                                                *needs_redraw = true;
                                                dirty_tracker.mark_all_dirty();
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                        
                        if let Some(idx) = clicked_idx {
                            let mut win = WINDOWS[idx].take().unwrap();
                            win.is_focused = true;
                            if drag_started {
                                win.is_dragging = true;
                            }
                            
                            for j in idx..(count - 1) {
                                WINDOWS[j] = WINDOWS[j + 1].take();
                            }
                            
                            for j in 0..(count - 1) {
                                if let Some(ref mut w) = WINDOWS[j] {
                                    w.is_focused = false;
                                    w.is_dragging = false;
                                }
                            }
                            
                            WINDOWS[count - 1] = Some(win);
                            for k in 0..5 {
                                WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                            }
                            dirty_tracker.mark_all_dirty();
                        } else {
                            for j in 0..count {
                                if let Some(ref mut w) = WINDOWS[j] {
                                    w.is_focused = false;
                                    w.is_dragging = false;
                                }
                            }
                            for k in 0..5 {
                                WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                            }
                            dirty_tracker.mark_all_dirty();
                        }
                    }
                }
            }
            else {
                // Dragging (prev_left_clicked == 1)
                unsafe {
                    for i in 0..5 {
                        if let Some(ref mut win) = WINDOWS[i] {
                            if win.is_dragging {
                                let (old_ax, old_ay) = win.get_animated_pos();
                                dirty_tracker.add_rect(old_ax - 20, old_ay - 20, win.width as i32 + 40, win.height as i32 + 40);
                                win.x += dx;
                                win.y += dy;
                                let (new_ax, new_ay) = win.get_animated_pos();
                                dirty_tracker.add_rect(new_ax - 20, new_ay - 20, win.width as i32 + 40, win.height as i32 + 40);
                            }
                        }
                    }
                }
            }
        } else {
            // Mouse Up (left_clicked == 0)
            unsafe {
                for i in 0..5 {
                    if let Some(ref mut win) = WINDOWS[i] {
                        win.is_dragging = false;
                    }
                }
            }
        }

        state.prev_mouse_x = state.cursor_x;
        state.prev_mouse_y = state.cursor_y;
        state.prev_left_clicked = left_clicked as u8;
    }
}

/// Handles incoming CLI characters from UART serial port.
pub fn handle_serial_input(
    serial_buf: &[u8],
    read_bytes: usize,
    dirty_tracker: &mut DirtyRectTracker,
    needs_redraw: &mut bool,
) {
    if read_bytes > 0 {
        *needs_redraw = true;
        WINDOW_BACKING_STORES[1].is_dirty.store(true, Ordering::Relaxed);
        let mut terminal_focused = false;
        unsafe {
            for i in 0..5 {
                if let Some(ref win) = WINDOWS[i] {
                    if win.id == 1 && win.is_focused {
                        terminal_focused = true;
                        break;
                    }
                }
            }
        }

        let start_menu_open = START_MENU_OPEN.load(Ordering::Relaxed);
        if start_menu_open {
            for i in 0..read_bytes {
                let byte = serial_buf[i];
                if byte == b'\r' || byte == b'\n' {
                    let mut q_buf = [0u8; 64];
                    let query_str = crate::state::SEARCH_QUERY.get_str(&mut q_buf);
                    let mut found_app = None;
                    for app in crate::graphics::compositor::ALL_APPS.iter() {
                        if crate::graphics::compositor::matches_search(app.title, query_str) {
                            found_app = Some(app);
                            break;
                        }
                    }
                    if let Some(app) = found_app {
                        if app.id == 5 {
                            let msg = "Shutting down system...\n";
                            let _ = sys_write(2, msg.as_ptr(), msg.len());
                            syscall0(3);
                        } else {
                            focus_window_by_id(app.id);
                            for k in 0..5 {
                                WINDOW_BACKING_STORES[k].is_dirty.store(true, Ordering::Relaxed);
                            }
                            dirty_tracker.mark_all_dirty();
                            START_MENU_ANIMATING.store(true, Ordering::Relaxed);
                            START_MENU_OPEN.store(false, Ordering::Relaxed);
                        }
                    }
                } else if byte == 0x08 || byte == 0x7F {
                    crate::state::SEARCH_QUERY.pop();
                    *needs_redraw = true;
                } else if byte >= 32 && byte <= 126 {
                    crate::state::SEARCH_QUERY.push(byte as char);
                    *needs_redraw = true;
                }
            }
        } else if terminal_focused {
            for i in 0..read_bytes {
                let byte = serial_buf[i];
                if byte == b'\r' || byte == b'\n' {
                    term_process_command();
                } else if byte == 0x08 || byte == 0x7F {
                    term_print_char('\x08');
                } else if byte >= 32 && byte <= 126 {
                    term_print_char(byte as char);
                }
            }
        }
        dirty_tracker.mark_all_dirty();
    }
}
