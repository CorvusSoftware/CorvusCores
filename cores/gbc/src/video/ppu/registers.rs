// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Memory-mapped LCD control/status, scrolling and monochrome palettes.
//!
//! Scroll and palette registers are stored but not yet consumed by a renderer.
//! DMA ($FF46) and CGB palette RAM registers are intentionally unmapped.

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
            _ => return false,
        }
        true
    }
}
