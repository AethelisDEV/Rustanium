// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # File Manager State Management
//!
//! Manages active directory paths, item selections, view modes, scroll positions,
//! total item counts, and modal dialog input states without unsafe code.

use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU8, Ordering};
use crate::utils::StrbufWriter;
use crate::syscalls::{sys_open, sys_read, sys_write, sys_close, sys_mkdir, sys_remove};

/// Thread-safe active path, navigation, view mode, scroll, and modal state.
pub struct FileManagerState {
    /// Character byte array storing the active directory path (up to 256 bytes).
    pub path_buf: [AtomicU8; 256],
    /// Length of the current active directory path string.
    pub path_len: AtomicU32,
    /// Currently selected item index in the active directory view (-1 for none).
    pub selected_index: AtomicI32,
    /// Currently hovered item index under the mouse cursor (-1 for none).
    pub hovered_index: AtomicI32,
    /// Active modal dialog mode (0 = None, 1 = New Folder, 2 = New File, 3 = Confirm Delete).
    pub modal_mode: AtomicU32,
    /// Text input buffer for modal dialog text entry (up to 64 bytes).
    pub modal_input_buf: [AtomicU8; 64],
    /// Length of the text input in the modal dialog.
    pub modal_input_len: AtomicU32,
    /// Active view mode (0 = List View, 1 = Grid View).
    pub view_mode: AtomicU32,
    /// Vertical scroll offset in pixels for item listings.
    pub scroll_offset: AtomicI32,
    /// Total number of items in the current active directory.
    pub total_items: AtomicU32,
    /// Flag indicating if the user is currently dragging the scrollbar thumb.
    pub is_dragging_scrollbar: AtomicU32,
}

impl FileManagerState {
    /// Creates a new `FileManagerState` instance initialized at the root directory (`/`).
    pub const fn new() -> Self {
        let mut buf = [const { AtomicU8::new(0) }; 256];
        buf[0] = AtomicU8::new(b'/');
        Self {
            path_buf: buf,
            path_len: AtomicU32::new(1),
            selected_index: AtomicI32::new(-1),
            hovered_index: AtomicI32::new(-1),
            modal_mode: AtomicU32::new(0),
            modal_input_buf: [const { AtomicU8::new(0) }; 64],
            modal_input_len: AtomicU32::new(0),
            view_mode: AtomicU32::new(0),
            scroll_offset: AtomicI32::new(0),
            total_items: AtomicU32::new(0),
            is_dragging_scrollbar: AtomicU32::new(0),
        }
    }

    /// Reads the current active directory path string into the provided byte buffer slice.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable byte slice to receive the path bytes.
    ///
    /// # Returns
    ///
    /// A valid string slice representing the active path, or `"/"` if UTF-8 conversion fails.
    pub fn get_path<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let len = self.path_len.load(Ordering::Relaxed) as usize;
        let limit = core::cmp::min(len, buf.len());
        for i in 0..limit {
            buf[i] = self.path_buf[i].load(Ordering::Relaxed);
        }
        core::str::from_utf8(&buf[..limit]).unwrap_or("/")
    }

    /// Sets the active directory path to a specific path string slice.
    ///
    /// # Arguments
    ///
    /// * `new_path` - The target directory path string slice.
    pub fn set_path(&self, new_path: &str) {
        let bytes = new_path.as_bytes();
        let limit = core::cmp::min(bytes.len(), 256);
        for i in 0..limit {
            self.path_buf[i].store(bytes[i], Ordering::Relaxed);
        }
        self.path_len.store(limit as u32, Ordering::Relaxed);
        self.selected_index.store(-1, Ordering::Relaxed);
        self.hovered_index.store(-1, Ordering::Relaxed);
        self.scroll_offset.store(0, Ordering::Relaxed);
    }

    /// Appends a child directory name to the current active directory path.
    ///
    /// # Arguments
    ///
    /// * `dir_name` - The relative subdirectory name to enter.
    pub fn push_dir(&self, dir_name: &str) {
        let mut current_buf = [0u8; 256];
        let current = self.get_path(&mut current_buf);
        let mut new_buf = [0u8; 256];
        let mut writer = StrbufWriter::new(&mut new_buf);
        
        let trimmed_name = dir_name.trim_end_matches('/');
        if current == "/" {
            let _ = core::fmt::write(&mut writer, format_args!("/{}", trimmed_name));
        } else {
            let _ = core::fmt::write(&mut writer, format_args!("{}/{}", current, trimmed_name));
        }
        self.set_path(writer.as_str());
    }

    /// Navigates up to the parent directory (e.g., `/system/logs` -> `/system`, or `/system` -> `/`).
    pub fn pop_dir(&self) {
        let mut current_buf = [0u8; 256];
        let current = self.get_path(&mut current_buf);
        if current == "/" {
            return;
        }
        
        if let Some(last_slash_idx) = current.rfind('/') {
            if last_slash_idx == 0 {
                self.set_path("/");
            } else {
                let parent = &current[..last_slash_idx];
                self.set_path(parent);
            }
        } else {
            self.set_path("/");
        }
    }

    /// Toggles the view mode between List View (0) and Grid View (1).
    pub fn toggle_view_mode(&self) {
        let mode = self.view_mode.load(Ordering::Relaxed);
        let new_mode = if mode == 0 { 1 } else { 0 };
        self.view_mode.store(new_mode, Ordering::Relaxed);
        self.scroll_offset.store(0, Ordering::Relaxed);
    }

    /// Calculates the maximum scroll offset based on total items count, view mode, and container height.
    ///
    /// # Arguments
    ///
    /// * `container_h` - Height of the view area in pixels.
    /// * `list_width` - Width of the item list area in pixels.
    ///
    /// # Returns
    ///
    /// Maximum scroll limit in pixels (0 if items fit on screen).
    pub fn calculate_max_scroll(&self, container_h: i32, list_width: i32) -> i32 {
        let total = self.total_items.load(Ordering::Relaxed) as i32;
        let view_mode = self.view_mode.load(Ordering::Relaxed);
        let max_height = if view_mode == 0 {
            total * 28
        } else {
            let cols = ((list_width - 24) / 74).max(1);
            ((total + cols - 1) / cols) * 70
        };

        (max_height - container_h).max(0)
    }

    /// Scrolls down by `delta` pixels, clamped strictly to `max_scroll`.
    ///
    /// # Arguments
    ///
    /// * `delta` - Scroll delta in pixels.
    /// * `container_h` - View area height.
    /// * `list_width` - Item list width.
    pub fn scroll_down(&self, delta: i32, container_h: i32, list_width: i32) {
        let max_scroll = self.calculate_max_scroll(container_h, list_width);
        let current = self.scroll_offset.load(Ordering::Relaxed);
        let next = (current + delta).min(max_scroll).max(0);
        self.scroll_offset.store(next, Ordering::Relaxed);
    }

    /// Scrolls up by `delta` pixels, clamped to >= 0.
    ///
    /// # Arguments
    ///
    /// * `delta` - Scroll delta in pixels.
    pub fn scroll_up(&self, delta: i32) {
        let current = self.scroll_offset.load(Ordering::Relaxed);
        let next = (current - delta).max(0);
        self.scroll_offset.store(next, Ordering::Relaxed);
    }

    /// Opens a modal dialog by setting its mode and clearing the input buffer.
    ///
    /// # Arguments
    ///
    /// * `mode` - The modal mode index (1 = New Folder, 2 = New File, 3 = Confirm Delete).
    pub fn open_modal(&self, mode: u32) {
        self.modal_input_len.store(0, Ordering::Relaxed);
        self.modal_mode.store(mode, Ordering::Relaxed);
    }

    /// Closes any active modal dialog.
    pub fn close_modal(&self) {
        self.modal_mode.store(0, Ordering::Relaxed);
        self.modal_input_len.store(0, Ordering::Relaxed);
    }

    /// Appends a typed character to the active modal input buffer.
    ///
    /// # Arguments
    ///
    /// * `c` - The ASCII character to append.
    pub fn push_modal_char(&self, c: char) {
        let len = self.modal_input_len.load(Ordering::Relaxed) as usize;
        if len < 64 {
            let val = c as u32;
            if val <= 255 {
                self.modal_input_buf[len].store(val as u8, Ordering::Relaxed);
                self.modal_input_len.store((len + 1) as u32, Ordering::Relaxed);
            }
        }
    }

    /// Removes the last character from the active modal input buffer.
    pub fn pop_modal_char(&self) {
        let len = self.modal_input_len.load(Ordering::Relaxed) as usize;
        if len > 0 {
            self.modal_input_len.store((len - 1) as u32, Ordering::Relaxed);
        }
    }

    /// Reads the modal dialog text input into the provided byte slice buffer.
    ///
    /// # Arguments
    ///
    /// * `buf` - Target byte slice to store input string bytes.
    ///
    /// # Returns
    ///
    /// A string slice containing the user-entered text.
    pub fn get_modal_input<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let len = self.modal_input_len.load(Ordering::Relaxed) as usize;
        let limit = core::cmp::min(len, buf.len());
        for i in 0..limit {
            buf[i] = self.modal_input_buf[i].load(Ordering::Relaxed);
        }
        core::str::from_utf8(&buf[..limit]).unwrap_or("")
    }
}

/// Global atomic instance tracking the active File Manager state.
pub static FILE_MANAGER_STATE: FileManagerState = FileManagerState::new();

/// Executes the pending action based on the active modal mode and user input string.
pub fn execute_modal_action() {
    let mode = FILE_MANAGER_STATE.modal_mode.load(Ordering::Relaxed);
    let mut input_tmp = [0u8; 64];
    let input_text = FILE_MANAGER_STATE.get_modal_input(&mut input_tmp);

    if mode == 1 && !input_text.is_empty() {
        // Create New Directory
        let mut current_buf = [0u8; 256];
        let current = FILE_MANAGER_STATE.get_path(&mut current_buf);
        let mut full_path_buf = [0u8; 256];
        let mut writer = StrbufWriter::new(&mut full_path_buf);
        if current == "/" {
            let _ = core::fmt::write(&mut writer, format_args!("/{}", input_text));
        } else {
            let _ = core::fmt::write(&mut writer, format_args!("{}/{}", current, input_text));
        }
        let target_path = writer.as_str();
        let _ = sys_mkdir(target_path.as_ptr(), target_path.len());
    } else if mode == 2 && !input_text.is_empty() {
        // Create New File
        let mut current_buf = [0u8; 256];
        let current = FILE_MANAGER_STATE.get_path(&mut current_buf);
        let mut full_path_buf = [0u8; 256];
        let mut writer = StrbufWriter::new(&mut full_path_buf);
        if current == "/" {
            let _ = core::fmt::write(&mut writer, format_args!("/{}", input_text));
        } else {
            let _ = core::fmt::write(&mut writer, format_args!("{}/{}", current, input_text));
        }
        let target_path = writer.as_str();
        let fd = sys_open(target_path.as_ptr(), target_path.len(), 2);
        if fd != u64::MAX && fd < 16 {
            let init_msg = "// New File\n";
            let _ = sys_write(fd, init_msg.as_ptr(), init_msg.len());
            sys_close(fd);
        }
    } else if mode == 3 {
        // Confirm Delete Selected Item
        let selected_idx = FILE_MANAGER_STATE.selected_index.load(Ordering::Relaxed);
        if selected_idx >= 0 {
            let mut current_buf = [0u8; 256];
            let current_path = FILE_MANAGER_STATE.get_path(&mut current_buf);
            let fd = sys_open(current_path.as_ptr(), current_path.len(), 0);
            if fd != u64::MAX && fd < 16 {
                let mut dir_buf = [0u8; 1024];
                let bytes_read = sys_read(fd, dir_buf.as_mut_ptr(), 1024);
                sys_close(fd);
                if bytes_read != u64::MAX && bytes_read > 0 {
                    let slice = &dir_buf[..bytes_read as usize];
                    if let Ok(s) = core::str::from_utf8(slice) {
                        for (idx, entry) in s.lines().enumerate() {
                            if idx as i32 == selected_idx {
                                let mut full_path_buf = [0u8; 256];
                                let mut writer = StrbufWriter::new(&mut full_path_buf);
                                let entry_clean = entry.trim_end_matches('/');
                                if current_path == "/" {
                                    let _ = core::fmt::write(&mut writer, format_args!("/{}", entry_clean));
                                } else {
                                    let _ = core::fmt::write(&mut writer, format_args!("{}/{}", current_path, entry_clean));
                                }
                                let target_path = writer.as_str();
                                let _ = sys_remove(target_path.as_ptr(), target_path.len());
                                break;
                            }
                        }
                    }
                }
            }
            FILE_MANAGER_STATE.selected_index.store(-1, Ordering::Relaxed);
        }
    }

    FILE_MANAGER_STATE.close_modal();
}

/// Handles keyboard input events targeted at the File Manager window or active modal.
///
/// # Arguments
///
/// * `key` - The decoded key code or ASCII value.
pub fn handle_file_manager_key(key: u32) {
    let mode = FILE_MANAGER_STATE.modal_mode.load(Ordering::Relaxed);
    if mode > 0 {
        if key == 0x1001 || key == 13 || key == 10 {
            // Enter key -> Submit modal action
            execute_modal_action();
        } else if key == 0x1000 || key == 8 {
            // Backspace key -> Delete last character
            FILE_MANAGER_STATE.pop_modal_char();
        } else if key == 0x101B || key == 27 {
            // Escape key -> Cancel modal
            FILE_MANAGER_STATE.close_modal();
        } else if key < 0x1000 {
            let c = (key as u8) as char;
            if c.is_ascii_graphic() || c == ' ' || c == '.' || c == '_' || c == '-' {
                FILE_MANAGER_STATE.push_modal_char(c);
            }
        }
    } else {
        // Standard Arrow & Page keys for scrolling when no modal is active
        let container_h = 200i32;
        let list_w = 200i32;
        if key == 0x1003 || key == 0x50 { // Down arrow
            FILE_MANAGER_STATE.scroll_down(28, container_h, list_w);
        } else if key == 0x1002 || key == 0x48 { // Up arrow
            FILE_MANAGER_STATE.scroll_up(28);
        } else if key == 0x1005 { // Page Down
            FILE_MANAGER_STATE.scroll_down(140, container_h, list_w);
        } else if key == 0x1004 { // Page Up
            FILE_MANAGER_STATE.scroll_up(140);
        }
    }
}

/// Determines a human-readable file type description based on the entry name and extension.
///
/// # Arguments
///
/// * `name` - The file or directory entry name string slice.
///
/// # Returns
///
/// A static string slice describing the item type.
pub fn detect_file_type(name: &str) -> &'static str {
    if name.ends_with('/') {
        "Folder"
    } else if name.ends_with(".rs") {
        "Rust Source Code"
    } else if name.ends_with(".txt") || name.ends_with(".md") {
        "Text Document"
    } else if name.ends_with(".log") {
        "System Log File"
    } else if name.ends_with(".bmp") || name.ends_with(".png") {
        "Bitmap Image"
    } else if name.ends_with(".elf") || name.ends_with(".bin") {
        "Executable Binary"
    } else if name.ends_with(".toml") || name.ends_with(".json") {
        "Configuration File"
    } else {
        "File"
    }
}
