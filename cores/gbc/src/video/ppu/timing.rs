// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Nominal LCD scanline timing and edge-triggered STAT interrupt line.

use super::{Mode, Ppu};
use crate::interrupts::{Interrupt, InterruptController};

const DOTS_PER_LINE: u16 = 456;
const OAM_DOTS: u16 = 80;
const TRANSFER_END_DOT: u16 = OAM_DOTS + 172;
const FIRST_VBLANK_LINE: u8 = 144;
const LAST_LINE: u8 = 153;

impl Ppu {
    /// Advance the PPU in LCD dots (one dot per normal-speed CPU T-cycle).
    ///
    /// No rendering/fetcher stalls, double-speed CGB conversion, or
    /// CPU VRAM/OAM access restrictions are modeled yet.
    pub(crate) fn tick(&mut self, dots: u32, interrupts: &mut InterruptController) {
        if !self.lcd_enabled() {
            return;
        }

        for _ in 0..dots {
            self.dot += 1;
            if self.dot == DOTS_PER_LINE {
                self.dot = 0;
                self.ly = if self.ly == LAST_LINE { 0 } else { self.ly + 1 };

                if self.ly == FIRST_VBLANK_LINE {
                    self.mode = Mode::VBlank;
                    interrupts.request(Interrupt::VBlank);
                } else if self.ly < FIRST_VBLANK_LINE {
                    self.mode = Mode::OamScan;
                }
                self.update_stat_line(interrupts);
            } else if self.ly < FIRST_VBLANK_LINE {
                match self.dot {
                    OAM_DOTS => {
                        self.mode = Mode::Transfer;
                        self.update_stat_line(interrupts);
                    }
                    TRANSFER_END_DOT => {
                        self.mode = Mode::HBlank;
                        self.update_stat_line(interrupts);
                    }
                    _ => {}
                }
            }
        }
    }

    /// STAT sources are ORed onto a shared line. Only a low-to-high
    /// transition requests an IRQ, even when multiple sources overlap.
    pub(super) fn update_stat_line(&mut self, interrupts: &mut InterruptController) {
        let next = self.lcd_enabled()
            && ((self.ly == self.lyc && self.stat_enable & 0x40 != 0)
                || (self.mode == Mode::OamScan && self.stat_enable & 0x20 != 0)
                || (self.mode == Mode::VBlank && self.stat_enable & 0x10 != 0)
                || (self.mode == Mode::HBlank && self.stat_enable & 0x08 != 0));

        if next && !self.stat_line {
            interrupts.request(Interrupt::LcdStat);
        }
        self.stat_line = next;
    }
}
