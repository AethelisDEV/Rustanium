// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

use crate::utils::Align4096;
use core::sync::atomic::{AtomicI32, AtomicU32, AtomicBool, AtomicU8, Ordering};

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
pub static SHADOWS_ENABLED: AtomicBool = AtomicBool::new(false);
/// Global active settings tab selector (0 = Appearance, 1 = System, 2 = About).
pub static ACTIVE_SETTINGS_TAB: AtomicU32 = AtomicU32::new(0);


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

/// A thread-safe, lock-free state structure representing the search input query
/// typed by the user inside the Applications Screen.
pub struct SearchQuery {
    /// Internal array of atomic bytes storing the characters of the query.
    pub buffer: [AtomicU8; 64],
    /// Atomic length storing the active length of the search string.
    pub len: AtomicU32,
}

impl SearchQuery {
    /// Creates and initializes a new empty `SearchQuery` instance.
    pub const fn new() -> Self {
        Self {
            buffer: [const { AtomicU8::new(0) }; 64],
            len: AtomicU32::new(0),
        }
    }

    /// Resets the query search input length to 0.
    pub fn clear(&self) {
        self.len.store(0, Ordering::Relaxed);
    }

    /// Appends a new character `c` to the query buffer.
    /// Does nothing if the query has reached its maximum size of 64 characters
    /// or if the character is non-ASCII.
    pub fn push(&self, c: char) {
        let len = self.len.load(Ordering::Relaxed) as usize;
        if len < 64 {
            let val = c as u32;
            if val <= 255 {
                self.buffer[len].store(val as u8, Ordering::Relaxed);
                self.len.store((len + 1) as u32, Ordering::Relaxed);
            }
        }
    }

    /// Removes the last character in the query buffer.
    /// Does nothing if the query buffer is already empty.
    pub fn pop(&self) {
        let len = self.len.load(Ordering::Relaxed) as usize;
        if len > 0 {
            self.len.store((len - 1) as u32, Ordering::Relaxed);
        }
    }

    /// Populates the given mutable byte slice with the active query character bytes
    /// and returns it as a string slice. If the buffer is invalid UTF-8, returns an empty string.
    pub fn get_str<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let len = self.len.load(Ordering::Relaxed) as usize;
        let limit = core::cmp::min(len, buf.len());
        for i in 0..limit {
            buf[i] = self.buffer[i].load(Ordering::Relaxed);
        }
        core::str::from_utf8(&buf[..limit]).unwrap_or("")
    }
}

/// Global atomic instance tracking the search query on the Applications Screen.
pub static SEARCH_QUERY: SearchQuery = SearchQuery::new();

