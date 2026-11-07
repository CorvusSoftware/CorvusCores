// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared CGB background / object palette RAM, storing RGB555 colors.
//!
//! Each palette RAM initializes to deterministic white for tests, not a simulated
//! CGB boot state. During pixel transfer, data reads return $FF and writes
//! do not modify RAM, but still advance the palette index if auto-increment is enabled.

#[derive(Debug)]
pub(super) struct CgbPaletteRam {
    data: [u8; 64],
    index: u8,
}

impl CgbPaletteRam {
    pub(super) fn new() -> Self {
        Self {
            data: [0xFF; 64],
            index: 0,
        }
    }

    pub(super) fn read_index(&self) -> u8 {
        self.index
    }

    pub(super) fn write_index(&mut self, value: u8) {
        self.index = value & 0xBF;
    }

    pub(super) fn read_data(&self, blocked: bool) -> u8 {
        if blocked {
            0xFF
        } else {
            self.data[usize::from(self.index & 0x3F)]
        }
    }

    pub(super) fn write_data(&mut self, value: u8, blocked: bool) {
        if !blocked {
            self.data[usize::from(self.index & 0x3F)] = value;
        }
        if self.index & 0x80 != 0 {
            self.index = 0x80 | (self.index.wrapping_add(1) & 0x3F);
        }
    }

    pub(super) fn color(&self, palette: u8, color: u8) -> u16 {
        let offset = usize::from(palette) * 8 + usize::from(color) * 2;
        u16::from_le_bytes([self.data[offset], self.data[offset + 1]]) & 0x7FFF
    }
}
