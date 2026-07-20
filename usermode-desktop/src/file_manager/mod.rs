// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # File Manager Module
//!
//! Provides a modular desktop file manager application split into state management,
//! visual drawing routines, and user interaction handlers.

pub mod state;
pub mod draw;
pub mod input;

pub use state::{FILE_MANAGER_STATE, FileManagerState, execute_modal_action, handle_file_manager_key, detect_file_type};
pub use draw::draw_file_manager;
pub use input::{handle_file_manager_click, handle_file_manager_mouse_drag};
