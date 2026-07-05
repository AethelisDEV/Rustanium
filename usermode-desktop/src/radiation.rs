// SPDX-License-Identifier: GPL-2.0-only
// Copyright (c) 2026 AethelisDEV / Rustix OS. All rights reserved.

//! # Radiation Simulator Application
//!
//! Bağımsız bir pencere uygulamasıdır. Kullanıcının fiziksel bellek çerçevelerine
//! kozmik ışın kaynaklı bit flip zararı enjekte etmesini sağlar. Enjeksiyon sonuçları
//! (ECC düzeltme, karantina, yer değiştirme) pencere içindeki canlı log terminalinde
//! ve seri UART çıkışında görünür.

use crate::atlas_font::{draw_text_atlas, AtlasSize, AtlasWeight};
use crate::graphics::{draw_rounded_rect_alpha, draw_rounded_rect_outline_alpha};
use core::sync::atomic::Ordering;

/// Maximum number of log lines shown in the terminal panel.
const MAX_LOGS: usize = 4;
/// Maximum bytes per log message.
const LOG_LINE_LEN: usize = 64;

/// Circular log ring-buffer shared across the radiation simulator module.
static mut RADIATION_LOGS: [[u8; LOG_LINE_LEN]; MAX_LOGS] = [[0; LOG_LINE_LEN]; MAX_LOGS];
/// Number of entries currently stored in the ring buffer (capped at MAX_LOGS).
static mut RADIATION_LOG_COUNT: usize = 0;

/// Appends a new diagnostic message to the in-window log and COM1 UART.
///
/// Messages are stored in a rolling 4-line ring buffer. The oldest entry is
/// overwritten when the buffer is full. Simultaneously the message is echoed
/// to the QEMU COM1 serial port for host-side capture.
pub fn add_radiation_log(msg: &str) {
    unsafe {
        // Echo to serial port so the developer can trace events from the host
        crate::utils::serial_print("[RADIATION] ");
        crate::utils::serial_print(msg);
        crate::utils::serial_print("\n");

        // Shift existing entries up to make room at the bottom
        for i in 0..(MAX_LOGS - 1) {
            RADIATION_LOGS[i] = RADIATION_LOGS[i + 1];
        }

        // Write the new entry into the last slot
        let mut line = [0u8; LOG_LINE_LEN];
        let len = msg.len().min(LOG_LINE_LEN - 1);
        line[..len].copy_from_slice(&msg.as_bytes()[..len]);
        RADIATION_LOGS[MAX_LOGS - 1] = line;

        if RADIATION_LOG_COUNT < MAX_LOGS {
            RADIATION_LOG_COUNT += 1;
        }
    }
}

/// Renders the Radiation Simulator window contents.
///
/// Draws a description header, injection control card with FLIP 1 / FLIP 2 buttons,
/// live ECC / quarantine / relocation counters fed from the kernel shared-info page,
/// and a scrolling log terminal showing the last four kernel events.
///
/// # Arguments
///
/// * `ax` - Animated left X-coordinate of the window body.
/// * `ay` - Animated top Y-coordinate of the window body.
/// * `w`  - Window width in pixels.
/// * `_h` - Window height in pixels (unused directly).
pub fn draw_radiation_window(ax: i32, ay: i32, w: usize, _h: usize) {
    // ── Header ──────────────────────────────────────────────────────────
    draw_text_atlas(ax + 24, ay + 50, "Radiation Memory Fault Injector", 230, 235, 245, AtlasSize::Small, AtlasWeight::SemiBold);
    draw_text_atlas(ax + 24, ay + 70, "Simulate cosmic-ray bit flips and watch kernel self-healing live.", 140, 148, 162, AtlasSize::Small, AtlasWeight::Regular);

    // ── Injection Control Card ───────────────────────────────────────────
    let card_y = ay + 100;
    let card_w = w as i32 - 48;
    draw_rounded_rect_alpha(ax + 24, card_y, card_w, 52, 8, 32, 34, 42, 235);
    draw_rounded_rect_outline_alpha(ax + 24, card_y, card_w, 52, 8, 60, 64, 80, 1, 100);

    draw_text_atlas(ax + 40, card_y + 17, "Inject Physical Bit Flip", 215, 222, 235, AtlasSize::Small, AtlasWeight::Regular);

    // FLIP 1 button (orange)
    let btn1_x = ax + w as i32 - 160;
    let btn1_y = card_y + 12;
    draw_rounded_rect_alpha(btn1_x, btn1_y, 56, 28, 6, 230, 140, 40, 255);
    draw_text_atlas(btn1_x + 9, btn1_y + 7, "FLIP 1", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

    // FLIP 2 button (red)
    let btn2_x = ax + w as i32 - 96;
    let btn2_y = card_y + 12;
    draw_rounded_rect_alpha(btn2_x, btn2_y, 56, 28, 6, 220, 60, 60, 255);
    draw_text_atlas(btn2_x + 9, btn2_y + 7, "FLIP 2", 255, 255, 255, AtlasSize::Small, AtlasWeight::SemiBold);

    // ── Live ECC Metrics Bar ─────────────────────────────────────────────
    let info = crate::syscalls::sys_get_shared_info();
    let (ecc, quarantine, relocated) = unsafe {
        (
            (*info).ecc_corrections.load(Ordering::Relaxed),
            (*info).pages_quarantined.load(Ordering::Relaxed),
            (*info).pages_relocated.load(Ordering::Relaxed),
        )
    };

    // Metric boxes
    let metric_y = card_y + 62;
    let box_w = (card_w - 8) / 3;

    // ECC box
    draw_rounded_rect_alpha(ax + 24, metric_y, box_w, 38, 6, 26, 60, 26, 235);
    draw_text_atlas(ax + 32, metric_y + 5, "ECC Repairs", 100, 210, 100, AtlasSize::Small, AtlasWeight::Regular);
    draw_ecc_number(ax + 32, metric_y + 20, ecc);

    // Quarantine box
    let q_box_x = ax + 24 + box_w + 4;
    draw_rounded_rect_alpha(q_box_x, metric_y, box_w, 38, 6, 60, 40, 20, 235);
    draw_text_atlas(q_box_x + 8, metric_y + 5, "Quarantined", 230, 160, 60, AtlasSize::Small, AtlasWeight::Regular);
    draw_ecc_number(q_box_x + 8, metric_y + 20, quarantine);

    // Relocated box
    let r_box_x = ax + 24 + 2 * (box_w + 4);
    draw_rounded_rect_alpha(r_box_x, metric_y, box_w, 38, 6, 40, 30, 60, 235);
    draw_text_atlas(r_box_x + 8, metric_y + 5, "Relocated", 180, 130, 230, AtlasSize::Small, AtlasWeight::Regular);
    draw_ecc_number(r_box_x + 8, metric_y + 20, relocated);

    // ── Log Terminal Panel ───────────────────────────────────────────────
    let term_y = metric_y + 46;
    let term_h = 82;
    draw_rounded_rect_alpha(ax + 24, term_y, card_w, term_h, 8, 10, 12, 16, 255);
    draw_rounded_rect_outline_alpha(ax + 24, term_y, card_w, term_h, 8, 38, 42, 50, 1, 100);

    // Render stored log lines
    unsafe {
        let count = RADIATION_LOG_COUNT;
        for i in 0..count {
            let log_idx = MAX_LOGS - count + i;
            let line_bytes = &RADIATION_LOGS[log_idx];
            let mut len = 0;
            while len < LOG_LINE_LEN && line_bytes[len] != 0 {
                len += 1;
            }
            if let Ok(s) = core::str::from_utf8(&line_bytes[..len]) {
                let (r, g, b) = if s.contains("FLIP2") {
                    (235, 100, 100) // red for double-bit
                } else if s.contains("FLIP1") {
                    (230, 170, 60) // orange for single-bit
                } else {
                    (90, 200, 150) // green for status
                };
                draw_text_atlas(ax + 34, term_y + 10 + (i as i32) * 18, s, r, g, b, AtlasSize::Small, AtlasWeight::Regular);
            }
        }
    }
}

/// Helper that draws a u64 counter value as plain text without heap allocation.
fn draw_ecc_number(x: i32, y: i32, value: u64) {
    if value == 0 {
        draw_text_atlas(x, y, "0", 200, 210, 220, AtlasSize::Small, AtlasWeight::Regular);
        return;
    }
    // Build digit string in-place (max 20 digits for u64)
    let mut buf = [0u8; 20];
    let mut n = value;
    let mut len = 0usize;
    while n > 0 {
        buf[19 - len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    if let Ok(s) = core::str::from_utf8(&buf[20 - len..]) {
        draw_text_atlas(x, y, s, 200, 210, 220, AtlasSize::Small, AtlasWeight::Regular);
    }
}
