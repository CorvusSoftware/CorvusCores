// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! DIV/TIMA/TMA/TAC backed by an internal 16-bit divider.
//!
//! Clock this component explicitly in CPU T-cycles. On a falling edge of the
//! selected divider bit, TIMA advances; overflow schedules a reload and a
//! timer interrupt request four T-cycles later. This is not yet a full
//! hardware scheduler, and CGB model-specific TAC transition quirks and
//! bus-write timing during the reload cycle remain to be validated.

use crate::interrupts::{Interrupt, InterruptController};

#[cfg(test)]
mod tests;

#[derive(Debug, Default)]
pub struct Timer {
    divider: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    reload_delay: u8,
}

impl Timer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0xFF04 => Some((self.divider >> 8) as u8),
            0xFF05 => Some(self.tima),
            0xFF06 => Some(self.tma),
            0xFF07 => Some(0xF8 | self.tac),
            _ => None,
        }
    }

    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0xFF04 => {
                let old_input = self.input_high();
                self.divider = 0;
                if old_input {
                    self.advance_tima();
                }
            }
            0xFF05 => {
                // Writing TIMA before the delayed reload cancels it.
                self.tima = value;
                self.reload_delay = 0;
            }
            0xFF06 => self.tma = value,
            0xFF07 => {
                let was_enabled = self.tac & 0x04 != 0;
                let old_input = self.input_high();
                self.tac = value & 0x07;
                // Switching between active clock sources can make the
                // selected divider bit fall. Disabling/enabling has additional
                // model-specific behavior, so avoid fabricating that glitch.
                if was_enabled && self.tac & 0x04 != 0 && old_input && !self.input_high() {
                    self.advance_tima();
                }
            }
            _ => return false,
        }
        true
    }

    pub fn tick(&mut self, t_cycles: u32, interrupts: &mut InterruptController) {
        for _ in 0..t_cycles {
            if self.reload_delay > 0 {
                self.reload_delay -= 1;
                if self.reload_delay == 0 {
                    self.tima = self.tma;
                    interrupts.request(Interrupt::Timer);
                }
            }

            let old_input = self.input_high();
            self.divider = self.divider.wrapping_add(1);
            if old_input && !self.input_high() {
                self.advance_tima();
            }
        }
    }

    fn input_high(&self) -> bool {
        if self.tac & 0x04 == 0 {
            return false;
        }
        let bit = match self.tac & 0x03 {
            0 => 9, // every 1024 T-cycles
            1 => 3, // every 16 T-cycles
            2 => 5, // every 64 T-cycles
            3 => 7, // every 256 T-cycles
            _ => unreachable!("two-bit TAC clock selector"),
        };
        self.divider & (1 << bit) != 0
    }

    fn advance_tima(&mut self) {
        // A TIMA overflow holds zero for four T-cycles before reloading TMA.
        if self.reload_delay != 0 {
            return;
        }
        if self.tima == 0xFF {
            self.tima = 0;
            self.reload_delay = 4;
        } else {
            self.tima += 1;
        }
    }
}
