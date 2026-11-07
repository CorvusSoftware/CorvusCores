// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Window overlay and its visible-line counter.
//!
//! The window uses WX-7 and its own tile map, not SCX/SCY. The internal
//! vertical counter advances only when at least one window pixel is drawn.
//! This scanline renderer does not implement mid-dot WX/LCDC quirks.

use super::Ppu;
use crate::video::VideoRam;
use crate::SCREEN_WIDTH;

impl Ppu {
    pub(super) fn render_window_line(&mut self, vram: &VideoRam) {
        let enabled = self.lcdc & 0x20 != 0
            && (self.cgb_mode || self.lcdc & 0x01 != 0)
            && self.window_y_triggered
            && self.wy < 144
            && self.wx <= 166;
        if !enabled {
            return;
        }

        let origin_x = i16::from(self.wx) - 7;
        let map_base = if self.lcdc & 0x40 != 0 {
            0x9C00
        } else {
            0x9800
        };
        let line_offset = usize::from(self.ly) * SCREEN_WIDTH;
        let window_y = self.window_line;

        for screen_x in 0..SCREEN_WIDTH {
            let window_x = screen_x as i16 - origin_x;
            if window_x < 0 {
                continue;
            }

            let pixel = self.sample_bg_pixel(vram, map_base, window_x as u8, window_y);
            self.framebuffer[line_offset + screen_x] = pixel.rgb555;
            self.bg_color_ids[screen_x] = pixel.color_id;
            self.bg_priority[screen_x] = pixel.priority;
        }
        self.window_line = self.window_line.wrapping_add(1);
    }
}
