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
pub static SHADOWS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Backing store for caching pre-rendered window contents.
///
/// Utilizes a thread-safe, lock-free array of AtomicU32 variables to allow
/// interior mutability within 100% safe Rust without heap allocation.
pub struct WindowBackingStore {
    /// Flattened array of cached pixels in 32-bit ARGB format.
    pub pixels: [AtomicU32; 580 * 380],
    /// The active width of the cached window contents.
    pub width: AtomicU32,
    /// The active height of the cached window contents.
    pub height: AtomicU32,
    /// Flag signaling whether the cache is invalid and requires redrawing.
    pub is_dirty: AtomicBool,
}

/// Global cache array storing the backing frames for the 5 windows.
pub static WINDOW_BACKING_STORES: [WindowBackingStore; 5] = [
    WindowBackingStore {
        pixels: [const { AtomicU32::new(0) }; 580 * 380],
        width: AtomicU32::new(0),
        height: AtomicU32::new(0),
        is_dirty: AtomicBool::new(true),
    },
    WindowBackingStore {
        pixels: [const { AtomicU32::new(0) }; 580 * 380],
        width: AtomicU32::new(0),
        height: AtomicU32::new(0),
        is_dirty: AtomicBool::new(true),
    },
    WindowBackingStore {
        pixels: [const { AtomicU32::new(0) }; 580 * 380],
        width: AtomicU32::new(0),
        height: AtomicU32::new(0),
        is_dirty: AtomicBool::new(true),
    },
    WindowBackingStore {
        pixels: [const { AtomicU32::new(0) }; 580 * 380],
        width: AtomicU32::new(0),
        height: AtomicU32::new(0),
        is_dirty: AtomicBool::new(true),
    },
    WindowBackingStore {
        pixels: [const { AtomicU32::new(0) }; 580 * 380],
        width: AtomicU32::new(0),
        height: AtomicU32::new(0),
        is_dirty: AtomicBool::new(true),
    },
];
