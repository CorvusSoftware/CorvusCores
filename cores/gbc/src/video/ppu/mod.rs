// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! LCD control and simplified Game Boy PPU mode progression.
//!
//! Background pixels are rendered once per visible scanline, not per dot.
//! Mode 3 still lasts a fixed 172 dots. Window/sprites, fetch stalls,
//! LY=153's early rollover and LCD startup timing are not modeled.

mod background;
mod palette;
mod registers;
mod timing;

use background::WHITE;
use palette::BgPalette;

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[cfg(test)]
mod render_tests;
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
    cgb_mode: bool,
    bg_palette: BgPalette,
    framebuffer: [u16; SCREEN_WIDTH * SCREEN_HEIGHT],
}

impl Ppu {
    /// Deterministic LCD-off state; not a DMG/CGB power-on snapshot.
    pub(crate) fn new(cgb_mode: bool) -> Self {
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
            cgb_mode,
            bg_palette: BgPalette::new(),
            framebuffer: [WHITE; SCREEN_WIDTH * SCREEN_HEIGHT],
        }
    }

    fn lcd_enabled(&self) -> bool {
        self.lcdc & 0x80 != 0
    }

    /// Fixed-size row-major framebuffer; each u16 stores RGB555 (bit 15 is 0).
    pub(crate) fn framebuffer(&self) -> &[u16] {
        &self.framebuffer
    }
}

