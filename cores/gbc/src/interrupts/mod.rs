// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Interrupt request (IF) and enable (IE) registers.
//!
//! The CPU owns IME and interrupt vector dispatch at instruction boundaries.
//! Per-M-cycle interrupt acknowledge timing is not yet simulated. A pending
//! request is not the same as a serviced interrupt.

#[cfg(test)]
mod tests;

/// Hardware priority order, highest first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interrupt {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub const fn mask(self) -> u8 {
        match self {
            Self::VBlank => 0x01,
            Self::LcdStat => 0x02,
            Self::Timer => 0x04,
            Self::Serial => 0x08,
            Self::Joypad => 0x10,
        }
    }

    pub const fn vector(self) -> u16 {
        match self {
            Self::VBlank => 0x0040,
            Self::LcdStat => 0x0048,
            Self::Timer => 0x0050,
            Self::Serial => 0x0058,
            Self::Joypad => 0x0060,
        }
    }
}

/// This is explicitly a test-state initialization, not a boot-ROM snapshot.
#[derive(Debug, Default)]
pub struct InterruptController {
    requested: u8,
    enabled: u8,
}

impl InterruptController {
    pub fn new() -> Self {
        Self::default()
    }

    /// IF: unused high three bits read as 1.
    pub fn read_if(&self) -> u8 {
        0xE0 | self.requested
    }

    pub fn write_if(&mut self, value: u8) {
        self.requested = value & 0x1F;
    }

    /// IE: all eight bits can be stored, but only the lower five are active.
    pub fn read_ie(&self) -> u8 {
        self.enabled
    }

    pub fn write_ie(&mut self, value: u8) {
        self.enabled = value;
    }

    pub fn request(&mut self, interrupt: Interrupt) {
        self.requested |= interrupt.mask();
    }

    /// Clearing a request does not change IE.
    pub fn acknowledge(&mut self, interrupt: Interrupt) {
        self.requested &= !interrupt.mask();
    }

    pub fn pending_mask(&self) -> u8 {
        self.requested & self.enabled & 0x1F
    }

    /// Highest-priority enabled request, even if the CPU's IME is still off.
    pub fn highest_pending(&self) -> Option<Interrupt> {
        let mask = self.pending_mask();
        match mask.trailing_zeros() {
            0 => Some(Interrupt::VBlank),
            1 => Some(Interrupt::LcdStat),
            2 => Some(Interrupt::Timer),
            3 => Some(Interrupt::Serial),
            4 => Some(Interrupt::Joypad),
            _ => None,
        }
    }
}
