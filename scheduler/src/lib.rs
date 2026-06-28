// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

#![no_std]

//! # AE Rustanium Scheduler Crate
//!
//! This crate provides core process scheduling, task execution redundant voting,
//! and dynamic virtual page hot-swapping for the kernel.
//!
//! Under a strict **Zero Unsafe Policy**, it guarantees fault mitigation of computational
//! and memory-based bit flips.

extern crate alloc;

pub mod tmr;
pub mod dispatcher;

// Re-export core types for simplified external usage
pub use tmr::{TmrVoter, TmrStatus, TmrResult};
pub use dispatcher::{KernelProcess, TaskDispatcher};
