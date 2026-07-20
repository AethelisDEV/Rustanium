// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ScreenInfo {
    pub framebuffer_addr: u64,
    pub width: u64,
    pub height: u64,
    pub stride: u64,
    pub bytes_per_pixel: u64,
    pub format: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub event_type: u32,       // 0 = None, 1 = Keyboard, 2 = Mouse
    pub keyboard_key: u32,     // Decoded character or special key code
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_left_clicked: u32,
    pub mouse_right_clicked: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SharedSystemInfo {
    pub system_ticks: core::sync::atomic::AtomicU64,
    pub heap_free: core::sync::atomic::AtomicU64,
    pub heap_used: core::sync::atomic::AtomicU64,
    pub cpu_usage: core::sync::atomic::AtomicU64,
    pub ecc_corrections: core::sync::atomic::AtomicU64,
    pub pages_quarantined: core::sync::atomic::AtomicU64,
    pub pages_relocated: core::sync::atomic::AtomicU64,
}

// ------------------------------------------------------------
// Low-Level Syscall Wrappers compliant with System V AMD64 ABI
// ------------------------------------------------------------

pub fn syscall0(id: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") id,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }
    ret
}

pub fn syscall1(id: u64, arg1: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") id,
            in("rdi") arg1,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }
    ret
}

pub fn syscall2(id: u64, arg1: u64, arg2: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }
    ret
}

pub fn syscall3(id: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") id,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }
    ret
}

pub fn sys_map_fb(info: *mut ScreenInfo) -> u64 {
    syscall1(0x10, info as u64)
}

pub fn sys_wait_event(event: *mut InputEvent, timeout_ms: u64) -> u64 {
    syscall2(0x11, event as u64, timeout_ms)
}

pub fn sys_open(path: *const u8, len: usize, flags: u64) -> u64 {
    syscall3(0x20, path as u64, len as u64, flags)
}

pub fn sys_read(fd: u64, buf: *mut u8, len: usize) -> u64 {
    syscall3(0x21, fd, buf as u64, len as u64)
}

pub fn sys_write(fd: u64, buf: *const u8, len: usize) -> u64 {
    syscall3(0x22, fd, buf as u64, len as u64)
}

pub fn sys_close(fd: u64) -> u64 {
    syscall1(0x23, fd)
}

pub fn sys_mkdir(path: *const u8, len: usize) -> u64 {
    syscall2(0x24, path as u64, len as u64)
}

/// Unlinks / removes a file or directory node from the virtual file system.
///
/// # Arguments
///
/// * `path` - Raw pointer to the path string bytes.
/// * `len` - Length of the path string in bytes.
///
/// # Returns
///
/// `0` on success, or `u64::MAX` on error.
pub fn sys_remove(path: *const u8, len: usize) -> u64 {
    syscall2(0x25, path as u64, len as u64)
}

pub fn sys_get_shared_info() -> *const SharedSystemInfo {
    syscall0(0x30) as *const SharedSystemInfo
}

pub fn sys_inject_bit_flip(frame_index: u64, offset: u64, bit_index: u64) -> u64 {
    syscall3(0x40, frame_index, offset, bit_index)
}

