// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Modular Graphics Pipeline
//!
//! Subdivides drawing functions into distinct files for mathematical primitives,
//! text printing, drop-shadow clipping, vector icons, and backing store compositing.

pub mod core;
pub mod text;
pub mod shadow;
pub mod icons;
pub mod compositor;

pub use self::core::*;
pub use self::text::*;
pub use self::shadow::*;
pub use self::icons::*;
pub use self::compositor::*;
