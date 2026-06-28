// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

use crate::utils::Align4096;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicBool};

// ------------------------------------------------------------
// Double Buffer Software Renderer (Stored in BSS Segment)
// Support up to 4K resolution (3840 * 2160 * 3 BGR format = 24,883,200 bytes)
// ------------------------------------------------------------

pub static mut BACK_BUFFER: Align4096<[u8; 24_883_200]> = Align4096([0; 24_883_200]);
pub static mut WALLPAPER_CACHE: Align4096<[u8; 24_883_200]> = Align4096([0; 24_883_200]);

pub static SCREEN_WIDTH: AtomicI32 = AtomicI32::new(1280);
pub static SCREEN_HEIGHT: AtomicI32 = AtomicI32::new(720);
pub static SCREEN_FORMAT: AtomicU32 = AtomicU32::new(0); // 0 = Bgr, 1 = Rgb
pub static mut CPU_HISTORY: [u8; 40] = [0; 40];

pub static START_MENU_OPEN: AtomicBool = AtomicBool::new(false);
pub static START_MENU_ANIMATING: AtomicBool = AtomicBool::new(false);
pub static START_MENU_ANIM_PROGRESS: AtomicU32 = AtomicU32::new(0); // f32 bit representation
