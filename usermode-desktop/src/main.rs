// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

#![no_std]
#![no_main]

//! # User Space Desktop Environment Entry Point
//!
//! Orchestrates startup resolution checks, window setups, and routes incoming
//! events to the input/render submodules.

pub mod syscalls;
pub mod font;
pub mod graphics;
pub mod window;
pub mod console;
pub mod atlas_font;
pub mod utils;
pub mod state;
pub mod wallpaper;
pub mod monitor;
pub mod taskbar;
pub mod file_manager;
pub mod settings;
pub mod dirty;
pub mod render;
pub mod input;

use syscalls::*;
use graphics::init_nebula_wallpaper;
use window::{Window, WINDOWS};
use console::term_init;
use utils::{StrbufWriter, serial_print};
use state::{
    WINDOW_BACKING_STORES, SCREEN_WIDTH, SCREEN_HEIGHT, SCREEN_FORMAT,
    START_MENU_OPEN, START_MENU_ANIMATING, START_MENU_ANIM_PROGRESS, CPU_HISTORY,
};
use dirty::DirtyRectTracker;

use core::sync::atomic::Ordering;

#[link_section = ".text.start"]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        // Enforce 16-byte stack alignment conforming to System V AMD64 ABI before entering main loop
        core::arch::asm!(
            "and rsp, -16",
            "call main_rust",
            options(noreturn)
        );
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    struct StderrWriter;
    impl core::fmt::Write for StderrWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let _ = sys_write(2, s.as_ptr(), s.len());
            Ok(())
        }
    }

    let mut writer = StderrWriter;
    let _ = core::fmt::write(&mut writer, format_args!("\n!!! USERMODE DESKTOP PANIC !!!\n{}\n", info));
    loop {}
}

#[no_mangle]
extern "C" fn main_rust() -> ! {
    serial_print("[DE] Entered main_rust\n");
    let shared_info = sys_get_shared_info();
    serial_print("[DE] Shared info fetched\n");

    let mut screen_info = ScreenInfo {
        framebuffer_addr: 0,
        width: 0,
        height: 0,
        stride: 0,
        bytes_per_pixel: 0,
        format: 0,
    };
    let map_status = sys_map_fb(&mut screen_info);
    serial_print("[DE] Framebuffer mapping requested\n");
    if map_status != 0 {
        serial_print("[DE] Framebuffer mapping failed! Exiting...\n");
        syscall0(3);
    }

    // Çözünürlük Sınır Koruması: Max 3840x2160 (4K çözünürlük) backbuffer taşmasını önler
    let total_pixels = (screen_info.width as usize) * (screen_info.height as usize);
    if total_pixels * 3 > 24_883_200 {
        serial_print("[DE] Screen resolution exceeds max buffer size! Capping to 1920x1080.\n");
        screen_info.width = 1920;
        screen_info.height = 1080;
    }

    SCREEN_WIDTH.store(screen_info.width as i32, Ordering::Relaxed);
    SCREEN_HEIGHT.store(screen_info.height as i32, Ordering::Relaxed);
    SCREEN_FORMAT.store(screen_info.format, Ordering::Relaxed);
    {
        let mut buf = [0u8; 128];
        let mut w = StrbufWriter::new(&mut buf);
        let sw = SCREEN_WIDTH.load(Ordering::Relaxed);
        let sh = SCREEN_HEIGHT.load(Ordering::Relaxed);
        let fmt = SCREEN_FORMAT.load(Ordering::Relaxed);
        let _ = core::fmt::write(&mut w, format_args!("[DE] Screen: {}x{} format={}\n", sw, sh, fmt));
        serial_print(w.as_str());
    }

    serial_print("[DE] Generating nebula wallpaper...\n");
    init_nebula_wallpaper();
    serial_print("[DE] Wallpaper ready.\n");

    term_init();
    serial_print("[DE] Term initialized\n");

    unsafe {
        WINDOWS[0] = Some(Window {
            id: 0,
            title: "System Monitor",
            x: 100,
            y: 60,
            width: 520,
            height: 420,
            is_dragging: false,
            is_focused: false,
            is_open: false,
            is_maximized: false,
            prev_x: 100,
            prev_y: 60,
            prev_w: 520,
            prev_h: 420,
            is_animating: false,
            anim_progress: 100,
            anim_direction: true,
        });

        WINDOWS[1] = Some(Window {
            id: 2,
            title: "File Manager",
            x: 680,
            y: 80,
            width: 480,
            height: 360,
            is_dragging: false,
            is_focused: false,
            is_open: false,
            is_maximized: false,
            prev_x: 680,
            prev_y: 80,
            prev_w: 480,
            prev_h: 360,
            is_animating: false,
            anim_progress: 100,
            anim_direction: true,
        });

        WINDOWS[2] = Some(Window {
            id: 1,
            title: "Console",
            x: 200,
            y: 200,
            width: 580,
            height: 380,
            is_dragging: false,
            is_focused: false,
            is_open: false,
            is_maximized: false,
            prev_x: 200,
            prev_y: 200,
            prev_w: 580,
            prev_h: 380,
            is_animating: false,
            anim_progress: 100,
            anim_direction: true,
        });

        WINDOWS[3] = Some(Window {
            id: 3,
            title: "Settings",
            x: 320,
            y: 160,
            width: 440,
            height: 280,
            is_dragging: false,
            is_focused: false,
            is_open: false,
            is_maximized: false,
            prev_x: 320,
            prev_y: 160,
            prev_w: 440,
            prev_h: 280,
            is_animating: false,
            anim_progress: 100,
            anim_direction: true,
        });
    }

    let mut input_state = input::InputState::new();
    input_state.cursor_x = screen_info.width as i32 / 2;
    input_state.cursor_y = screen_info.height as i32 / 2;
    input_state.prev_mouse_x = input_state.cursor_x;
    input_state.prev_mouse_y = input_state.cursor_y;

    let mut prev_render_x: i32 = input_state.cursor_x;
    let mut prev_render_y: i32 = input_state.cursor_y;

    let mut serial_buf = [0u8; 16];

    let mut event = InputEvent {
        event_type: 0,
        keyboard_key: 0,
        mouse_x: 0,
        mouse_y: 0,
        mouse_left_clicked: 0,
        mouse_right_clicked: 0,
    };

    let mut needs_redraw = true;
    let mut dirty_tracker = DirtyRectTracker::new();
    dirty_tracker.mark_all_dirty();
    let mut last_tick_update = 0;
    let mut last_anim_tick = 0;

    loop {
        let got_event = sys_wait_event(&mut event, 2);
        let mut event_processed = false;

        if got_event == 1 {
            event_processed = true;
            loop {
                input::handle_input_event(&event, &mut input_state, &mut dirty_tracker, &mut needs_redraw);
                if sys_wait_event(&mut event, 0) == 1 {
                    continue;
                } else {
                    break;
                }
            }
        }

        let read_bytes = sys_read(0, serial_buf.as_mut_ptr(), 16);
        if read_bytes > 0 && read_bytes != u64::MAX {
            event_processed = true;
            input::handle_serial_input(&serial_buf, read_bytes as usize, &mut dirty_tracker, &mut needs_redraw);
        }

        let ticks = unsafe { (*shared_info).system_ticks.load(Ordering::Relaxed) };
        if ticks - last_tick_update >= 10 { // Update metrics every 100ms
            last_tick_update = ticks;
            needs_redraw = true;
            WINDOW_BACKING_STORES[0].is_dirty.store(true, Ordering::Relaxed);
            unsafe {
                let cpu_load = ((*shared_info).cpu_usage.load(Ordering::Relaxed) / 100) as u8;
                for i in 0..39 {
                    CPU_HISTORY[i] = CPU_HISTORY[i+1];
                }
                CPU_HISTORY[39] = cpu_load;
            }
        }

        // Handle animation step
        let mut anim_running = false;
        unsafe {
            for i in 0..4 {
                if let Some(ref mut win) = WINDOWS[i] {
                    if win.is_animating {
                        anim_running = true;
                    }
                }
            }
        }
        if START_MENU_ANIMATING.load(Ordering::Relaxed) {
            anim_running = true;
        }

        if anim_running && ticks - last_anim_tick >= 1 {
            last_anim_tick = ticks;
            
            // 1. Windows animation steps
            unsafe {
                for i in 0..4 {
                    if let Some(ref mut win) = WINDOWS[i] {
                        if win.is_animating {
                            if win.anim_direction {
                                if win.anim_progress < 100 {
                                    win.anim_progress += 10;
                                } else {
                                    win.is_animating = false;
                                    win.is_focused = true;
                                }
                            } else {
                                if win.anim_progress > 0 {
                                    win.anim_progress -= 10;
                                } else {
                                    win.is_animating = false;
                                    win.is_open = false;
                                    win.is_focused = false;
                                }
                            }
                        }
                    }
                }
            }
            
            // 2. Start menu animation steps
            if START_MENU_ANIMATING.load(Ordering::Relaxed) {
                let current_progress_bits = START_MENU_ANIM_PROGRESS.load(Ordering::Relaxed);
                let mut current_progress = f32::from_bits(current_progress_bits);
                let open = START_MENU_OPEN.load(Ordering::Relaxed);
                
                if open {
                    if current_progress < 1.0 {
                        current_progress += 0.15;
                        if current_progress >= 1.0 {
                            current_progress = 1.0;
                            START_MENU_ANIMATING.store(false, Ordering::Relaxed);
                        }
                    }
                } else {
                    if current_progress > 0.0 {
                        current_progress -= 0.15;
                        if current_progress <= 0.0 {
                            current_progress = 0.0;
                            START_MENU_ANIMATING.store(false, Ordering::Relaxed);
                        }
                    }
                }
                START_MENU_ANIM_PROGRESS.store(current_progress.to_bits(), Ordering::Relaxed);
            }
        } else if !anim_running {
            last_anim_tick = ticks;
        }

        if anim_running {
            needs_redraw = true;
        }

        // Render pass
        let needs_draw_frame = needs_redraw || input_state.cursor_x != prev_render_x || input_state.cursor_y != prev_render_y;
        if needs_draw_frame || event_processed {
            render::draw_compositor_frame(
                &screen_info,
                &mut dirty_tracker,
                shared_info,
                input_state.cursor_x,
                input_state.cursor_y,
                &mut prev_render_x,
                &mut prev_render_y,
                ticks,
                anim_running,
                &mut needs_redraw,
            );
        }
    }
}
