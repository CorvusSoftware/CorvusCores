// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! FF46 OAM DMA request and transfer clock.
//!
//! A transfer copies 160 bytes from XX00–XX9F to FE00–FE9F, one byte
//! every four normal-speed CPU T-cycles (640 T-cycles total). The initial
//! start-up latency, bus conflicts, open-bus values, and double-speed CGB
//! behavior are not yet modeled. A new FF46 write restarts the transfer.

#[cfg(test)]
mod tests;
mod vram;
#[cfg(test)]
mod vram_tests;

pub(crate) use vram::VramDma;

const OAM_LENGTH: u16 = 160;
const T_CYCLES_PER_BYTE: u8 = 4;

#[derive(Debug, Default)]
pub(crate) struct OamDma {
    source_high: u8,
    copied: u16,
    phase: u8,
    active: bool,
}

impl OamDma {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// FF46 retains the last written page. Zero is only a test-state default.
    pub(crate) fn source_high(&self) -> u8 {
        self.source_high
    }

    pub(crate) fn active(&self) -> bool {
        self.active
    }

    pub(crate) fn start(&mut self, page: u8) {
        self.source_high = page;
        self.copied = 0;
        self.phase = 0;
        self.active = true;
    }

    /// Return the next (source, destination) address pair after four T-cycles.
    /// Only the DMA unit's state is changed; the bus performs the actual copy.
    pub(crate) fn tick_t_cycle(&mut self) -> Option<(u16, u16)> {
        if !self.active {
            return None;
        }
        self.phase += 1;
        if self.phase != T_CYCLES_PER_BYTE {
            return None;
        }
        self.phase = 0;

        let index = self.copied;
        self.copied += 1;
        if self.copied == OAM_LENGTH {
            self.active = false;
        }

        let source = (u16::from(self.source_high) << 8) | index;
        let destination = 0xFE00 + index;
        Some((source, destination))
    }
}
