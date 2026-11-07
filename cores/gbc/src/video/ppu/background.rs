// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared tile sampling for the background and window scanline renderer.
//!
//! Tile map indices come from VRAM bank 0; CGB attributes come from bank 1.
//! This does not model mid-scanline changes or the pixel fetcher.

use super::Ppu;
use crate::video::VideoRam;
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub(super) const WHITE: u16 = 0x7FFF;

// Deterministic presentation shades, not a calibrated Game Boy display.
pub(super) const DMG_SHADES: [u16; 4] = [WHITE, 0x56B5, 0x294A, 0x0000];

pub(super) struct BgPixel {
    pub(super) color_id: u8,
    pub(super) priority: bool,
    pub(super) rgb555: u16,
}

impl Ppu {
    pub(super) fn sample_bg_pixel(&self, vram: &VideoRam, map_base: u16, x: u8, y: u8) -> BgPixel {
        let map_address = map_base + u16::from(y / 8) * 32 + u16::from(x / 8);
        let tile = vram
            .read_bank(map_address, 0)
            .expect("background/window tile map address");
        let attributes = if self.cgb_mode {
            vram.read_bank(map_address, 1)
                .expect("CGB tile attributes map address")
        } else {
            0
        };
        let bank = (attributes >> 3) & 1;
        let tile_x = if attributes & 0x20 != 0 {
            7 - (x & 7)
        } else {
            x & 7
        };
        let tile_y = if attributes & 0x40 != 0 {
            7 - (y & 7)
        } else {
            y & 7
        };

        let tile_base = if self.lcdc & 0x10 != 0 {
            0x8000 + u16::from(tile) * 16
        } else {
            (0x9000_i32 + i32::from(tile as i8) * 16) as u16
        };
        let address = tile_base + u16::from(tile_y) * 2;
        let low = vram.read_bank(address, bank).expect("tile low byte");
        let high = vram.read_bank(address + 1, bank).expect("tile high byte");
        let bit = 7 - tile_x;
        let color_id = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);
        let rgb555 = if self.cgb_mode {
            self.bg_palette.color(attributes & 7, color_id)
        } else {
            let shade = (self.bgp >> (color_id * 2)) & 3;
            DMG_SHADES[usize::from(shade)]
        };
        BgPixel {
            color_id,
            priority: attributes & 0x80 != 0,
            rgb555,
        }
    }

    pub(super) fn render_background_line(&mut self, vram: &VideoRam) {
        let line = usize::from(self.ly);
        debug_assert!(line < SCREEN_HEIGHT);

        // LCDC.0 blanks BG and window in DMG compatibility. Native CGB
        // still renders both, but removes their priority over objects.
        if !self.cgb_mode && self.lcdc & 0x01 == 0 {
            self.framebuffer[line * SCREEN_WIDTH..(line + 1) * SCREEN_WIDTH].fill(WHITE);
            self.bg_color_ids.fill(0);
            self.bg_priority.fill(false);
            return;
        }

        let map_base = if self.lcdc & 0x08 != 0 {
            0x9C00
        } else {
            0x9800
        };
        let y = self.scy.wrapping_add(self.ly);
        for screen_x in 0..SCREEN_WIDTH {
            let x = self.scx.wrapping_add(screen_x as u8);
            let pixel = self.sample_bg_pixel(vram, map_base, x, y);
            self.framebuffer[line * SCREEN_WIDTH + screen_x] = pixel.rgb555;
            self.bg_color_ids[screen_x] = pixel.color_id;
            self.bg_priority[screen_x] = pixel.priority;
        }
    }
}
