// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! LCD control and simplified Game Boy PPU mode progression.
//!
//! This is NOT a renderer. Mode 3 uses its minimum 172-dot duration;
//! scroll/window/sprite fetch stalls, LY=153's early rollover,
//! memory arbitration and initial LCD startup timing are not modeled.

mod registers;
mod timing;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Transfer = 3,
}

/// PPU register state and nominal LCD dot clock.
#[derive(Debug)]
pub(crate) struct Ppu {
    lcdc: u8,
    stat_enable: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    dot: u16,
    mode: Mode,
    stat_line: bool,
}

impl Ppu {
    /// Deterministic LCD-off state; not a DMG/CGB power-on snapshot.
    pub(crate) fn new() -> Self {
        Self {
            lcdc: 0,
            stat_enable: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            dot: 0,
            mode: Mode::HBlank,
            stat_line: false,
        }
    }

    fn lcd_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}
