// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! DMG/CGB object selection and scanline composition.
//!
//! OAM selects the first ten objects overlapping LY, even when their X
//! coordinate hides them. DMG drawing prefers smaller X, then OAM order;
//! CGB prefers OAM order. No OAM scan timing or fetch stalls are modeled.

use super::background::DMG_SHADES;
use super::Ppu;
use crate::video::{ObjectRam, VideoRam};
use crate::SCREEN_WIDTH;

#[derive(Clone, Copy)]
struct Sprite {
    x: u8,
    y: u8,
    tile: u8,
    attributes: u8,
    index: usize,
}

impl Ppu {
    pub(super) fn render_objects_line(&mut self, vram: &VideoRam, oam: &ObjectRam) {
        if self.lcdc & 0x02 == 0 {
            return;
        }

        let height: i16 = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
        let mut sprites = [None; 10];
        let mut count = 0;
        for index in 0..40 {
            let address = 0xFE00 + (index as u16) * 4;
            let y = oam.read(address).expect("OAM sprite Y is mapped");
            let sprite_top = i16::from(y) - 16;
            let row = i16::from(self.ly) - sprite_top;
            if !(0..height).contains(&row) {
                continue;
            }
            sprites[count] = Some(Sprite {
                y,
                x: oam.read(address + 1).expect("OAM sprite X is mapped"),
                tile: oam.read(address + 2).expect("OAM sprite tile is mapped"),
                attributes: oam.read(address + 3).expect("OAM attributes are mapped"),
                index,
            });
            count += 1;
            if count == 10 {
                break;
            }
        }

        let line_offset = usize::from(self.ly) * SCREEN_WIDTH;
        for screen_x in 0..SCREEN_WIDTH {
            let mut candidate: Option<(Sprite, u8)> = None;
            for sprite in sprites.iter().flatten() {
                let pixel_x = screen_x as i16 - (i16::from(sprite.x) - 8);
                if !(0..8).contains(&pixel_x) {
                    continue;
                }

                let sprite_y = i16::from(self.ly) - (i16::from(sprite.y) - 16);
                let pixel_y = if sprite.attributes & 0x40 != 0 {
                    height - 1 - sprite_y
                } else {
                    sprite_y
                };
                let tile = if height == 16 {
                    (sprite.tile & 0xFE).wrapping_add((pixel_y / 8) as u8)
                } else {
                    sprite.tile
                };
                let x = if sprite.attributes & 0x20 != 0 {
                    7 - pixel_x as u8
                } else {
                    pixel_x as u8
                };
                let row = (pixel_y % 8) as u16;
                let bank = if self.cgb_mode {
                    (sprite.attributes >> 3) & 1
                } else {
                    0
                };
                let tile_address = 0x8000 + u16::from(tile) * 16 + row * 2;
                let low = vram
                    .read_bank(tile_address, bank)
                    .expect("OBJ tile low byte");
                let high = vram
                    .read_bank(tile_address + 1, bank)
                    .expect("OBJ tile high byte");
                let bit = 7 - x;
                let color_id = ((low >> bit) & 1) | (((high >> bit) & 1) << 1);
                if color_id == 0 {
                    continue; // OBJ color 0 is transparent, not an opaque winner.
                }

                let replace = match candidate {
                    None => true,
                    Some((previous, _)) if !self.cgb_mode => {
                        (sprite.x, sprite.index) < (previous.x, previous.index)
                    }
                    Some((previous, _)) => sprite.index < previous.index,
                };
                if replace {
                    candidate = Some((*sprite, color_id));
                }
            }

            let Some((sprite, color_id)) = candidate else {
                continue;
            };
            // Resolve OBJ-vs-OBJ *before* deciding whether BG hides the
            // winning object. A hidden high-priority object masks lower ones.
            let bg_opaque = self.bg_color_ids[screen_x] != 0;
            let hidden = if self.cgb_mode {
                self.lcdc & 1 != 0
                    && bg_opaque
                    && (self.bg_priority[screen_x] || sprite.attributes & 0x80 != 0)
            } else {
                bg_opaque && sprite.attributes & 0x80 != 0
            };
            if hidden {
                continue;
            }
            let color = if self.cgb_mode {
                self.obj_palette.color(sprite.attributes & 7, color_id)
            } else {
                let palette = if sprite.attributes & 0x10 != 0 {
                    self.obp1
                } else {
                    self.obp0
                };
                let shade = (palette >> (2 * color_id)) & 3;
                DMG_SHADES[usize::from(shade)]
            };
            self.framebuffer[line_offset + screen_x] = color;
        }
    }
}
