// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! LCD status, scrolling, DMG palettes, and CGB background palette RAM.
//!
//! DMA ($FF46) and CGB object palettes ($FF6A/$FF6B) remain unmapped.

use super::background::WHITE;
use super::{Mode, Ppu};
use crate::interrupts::InterruptController;

impl Ppu {
    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF40 => Some(self.lcdc),
            0xFF41 => Some(
                0x80 | self.stat_enable
                    | if self.ly == self.lyc { 0x04 } else { 0 }
                    | self.mode as u8,
            ),
            0xFF42 => Some(self.scy),
            0xFF43 => Some(self.scx),
            0xFF44 => Some(self.ly),
            0xFF45 => Some(self.lyc),
            0xFF47 => Some(self.bgp),
            0xFF48 => Some(self.obp0),
            0xFF49 => Some(self.obp1),
            0xFF4A => Some(self.wy),
            0xFF4B => Some(self.wx),
            0xFF68 if self.cgb_mode => Some(self.bg_palette.read_index()),
            0xFF69 if self.cgb_mode => Some(
                self.bg_palette
                    .read_data(self.lcd_enabled() && self.mode == Mode::Transfer),
            ),
            _ => None,
        }
    }

    pub(crate) fn write(
        &mut self,
        address: u16,
        value: u8,
        interrupts: &mut InterruptController,
    ) -> bool {
        match address {
            0xFF40 => {
                let was_enabled = self.lcd_enabled();
                self.lcdc = value;
                if was_enabled != self.lcd_enabled() {
                    self.ly = 0;
                    self.dot = 0;
                    self.mode = if self.lcd_enabled() {
                        Mode::OamScan
                    } else {
                        Mode::HBlank
                    };
                    self.stat_line = false;
                    if !self.lcd_enabled() {
                        self.framebuffer.fill(WHITE);
                    }
                    self.update_stat_line(interrupts);
                }
            }
            0xFF41 => {
                // Only interrupt selection bits 3–6 are writable.
                self.stat_enable = value & 0x78;
                self.update_stat_line(interrupts);
            }
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => {} // LY is read-only.
            0xFF45 => {
                self.lyc = value;
                self.update_stat_line(interrupts);
            }
            0xFF47 => self.bgp = value,
            0xFF48 => self.obp0 = value,
            0xFF49 => self.obp1 = value,
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            0xFF68 if self.cgb_mode => self.bg_palette.write_index(value),
            0xFF69 if self.cgb_mode => self
                .bg_palette
                .write_data(value, self.lcd_enabled() && self.mode == Mode::Transfer),
            _ => return false,
        }
        true
    }
}
