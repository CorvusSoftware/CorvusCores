// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! First framebuffer renderer: scrolling background tiles, no window/sprites.
//!
//! One entire scanline is sampled when nominal mode 3 ends (dot 252).
//! This does not model mid-scanline register writes or variable fetch timing.
//! CGB BG priority attributes are retained in VRAM but will matter only when
//! objects are implemented. RGB555 is a transport format, not LCD color science.

use super::Ppu;
use crate::video::VideoRam;
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

pub(super) const WHITE: u16 = 0x7FFF;

// Deterministic presentation shades, not a calibrated Game Boy display.
const DMG_SHADES: [u16; 4] = [WHITE, 0x56B5, 0x294A, 0x0000];

impl Ppu {
    pub(super) fn render_background_line(&mut self, vram: &VideoRam) {
        let line = usize::from(self.ly);
        debug_assert!(line < SCREEN_HEIGHT);

        // LCDC.0 disables the background in DMG compatibility. On a native
        // CGB it only affects BG-to-OBJ priority; background stays visible.
        if !self.cgb_mode && self.lcdc & 0x01 == 0 {
            self.framebuffer[line * SCREEN_WIDTH..(line + 1) * SCREEN_WIDTH].fill(WHITE);
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
            let map_address = map_base + u16::from(y / 8) * 32 + u16::from(x / 8);
            let tile = vram
                .read_bank(map_address, 0)
                .expect("background map address is valid");

            let attributes = if self.cgb_mode {
                vram.read_bank(map_address, 1)
                    .expect("CGB attribute map address is valid")
            } else {
                0
            };

            let tile_bank = (attributes >> 3) & 1;
            let mut tile_x = x & 7;
            let mut tile_y = y & 7;
            if attributes & 0x20 != 0 {
                tile_x = 7 - tile_x;
            }
            if attributes & 0x40 != 0 {
                tile_y = 7 - tile_y;
            }

            let tile_base = if self.lcdc & 0x10 != 0 {
                0x8000 + u16::from(tile) * 16
            } else {
                (0x9000_i32 + i32::from(tile as i8) * 16) as u16
            };
            let address = tile_base + u16::from(tile_y) * 2;
            let low = vram.read_bank(address, tile_bank).expect("valid tile row");
            let high = vram
                .read_bank(address + 1, tile_bank)
                .expect("valid tile row");
            let bit = 7 - tile_x;
            let color_id = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);

            let rgb555 = if self.cgb_mode {
                self.bg_palette.color(attributes & 7, color_id)
            } else {
                let shade = (self.bgp >> (color_id * 2)) & 3;
                DMG_SHADES[usize::from(shade)]
            };
            self.framebuffer[line * SCREEN_WIDTH + screen_x] = rgb555;
        }
    }
}
