// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Dirty Rectangles Tracking Module
//!
//! Provides a safe, non-allocating, fixed-size tracker to record regions of the
//! screen that have changed and require copying from the back buffer to the
//! physical framebuffer.

/// A simple rectangle representation for tracking screen updates.
#[derive(Clone, Copy, Debug)]
pub struct DirtyRect {
    /// Starting X-coordinate of the dirty region.
    pub x: i32,
    /// Starting Y-coordinate of the dirty region.
    pub y: i32,
    /// Width of the dirty region.
    pub w: i32,
    /// Height of the dirty region.
    pub h: i32,
}

/// Dynamic tracker for changed screen regions.
pub struct DirtyRectTracker {
    rects: [Option<DirtyRect>; 16],
    count: usize,
    all_dirty: bool,
}

impl DirtyRectTracker {
    /// Creates a new `DirtyRectTracker`.
    pub const fn new() -> Self {
        Self {
            rects: [None; 16],
            count: 0,
            all_dirty: false,
        }
    }

    /// Mark the entire screen as dirty, forcing a full frame copy.
    pub fn mark_all_dirty(&mut self) {
        self.all_dirty = true;
    }

    /// Reset the tracker state.
    pub fn clear(&mut self) {
        self.count = 0;
        self.all_dirty = false;
        let mut i = 0;
        while i < 16 {
            self.rects[i] = None;
            i += 1;
        }
    }

    /// Add a dirty region to the tracker.
    pub fn add_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if self.all_dirty || w <= 0 || h <= 0 {
            return;
        }
        if self.count >= 16 {
            // Out of slots, fall back to redrawing the entire screen
            self.all_dirty = true;
            return;
        }
        self.rects[self.count] = Some(DirtyRect { x, y, w, h });
        self.count += 1;
    }

    /// Returns a slice of active dirty rectangles.
    pub fn get_rects(&self) -> &[Option<DirtyRect>] {
        &self.rects[0..self.count]
    }

    /// Returns whether the entire screen should be redrawn.
    pub fn is_all_dirty(&self) -> bool {
        self.all_dirty
    }
}
