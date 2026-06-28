// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

#![no_std]

extern crate alloc;

pub mod usermode;
pub mod syscall;

// Re-export items for easier access
pub use usermode::{
    PHYSICAL_MEMORY_OFFSET, KERNEL_SHELL_RSP, KERNEL_SHELL_RBP,
    USER_CODE_BASE, USER_STACK_TOP, USER_STACK_SIZE,
    map_page_user, map_page_user_readonly, create_user_page_mapping,
    execute_user_program, demonstrate_user_mode,
    virt_to_phys, create_user_page_mapping_readonly,
};
pub use syscall::{init_syscalls, SYSCALL_HANDLER};

use core::sync::atomic::{AtomicPtr, Ordering};

/// Global logger callback to allow clean decoupled logging back to the host kernel console.
pub static LOG_CALLBACK: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());

/// Sets the active logger callback function.
pub fn init_logger(callback: fn(&str)) {
    LOG_CALLBACK.store(callback as *mut core::ffi::c_void, Ordering::Release);
}

/// Dispatches a log message back to the registered console callback.
pub fn log(msg: &str) {
    let cb_ptr = LOG_CALLBACK.load(Ordering::Acquire);
    if !cb_ptr.is_null() {
        let cb: fn(&str) = unsafe { core::mem::transmute(cb_ptr) };
        cb(msg);
    }
}

/// Internal macro to simplify formatting and logging across usermode and syscall submodules.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log(&alloc::format!($($arg)*));
    };
}
