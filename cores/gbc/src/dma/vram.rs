// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! CGB VRAM DMA control registers FF51–FF55 (GDMA and HBlank DMA).
//!
//! Transfers 16-byte blocks into the VRAM bank selected by VBK, at one
//! byte per two normal-speed T-cycles. General-purpose DMA stalls the CPU
//! for the full transfer; HBlank DMA stalls it during each 32-T-cycle block.
//! CPU bus interactions are only modeled at instruction boundaries.
//! Invalid source pages, LCD startup, active-register writes, and CGB
//! double-speed timing are not yet hardware-complete.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransferMode {
    General,
    HBlank,
}

/// Register latches plus active transfer progress.
#[derive(Debug)]
pub(crate) struct VramDma {
    source_high: u8,
    source_low: u8,
    destination_high: u8,
    destination_low: u8,
    source: u16,
    destination: u16,
    blocks_left: u8,
    byte_in_block: u8,
    phase: u8,
    mode: TransferMode,
    active: bool,
    block_active: bool,
    stopped_status: u8,
}

impl VramDma {
    pub(crate) fn new() -> Self {
        Self {
            source_high: 0,
            source_low: 0,
            destination_high: 0,
            destination_low: 0,
            source: 0,
            destination: 0x8000,
            blocks_left: 0,
            byte_in_block: 0,
            phase: 0,
            mode: TransferMode::General,
            active: false,
            block_active: false,
            stopped_status: 0xFF,
        }
    }

    /// HDMA1–4 are write-only, and HDMA5 exposes progress.
    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        (address == 0xFF55).then(|| {
            if self.active {
                self.blocks_left - 1
            } else {
                self.stopped_status
            }
        })
    }

    /// Returns true for supported transfer control registers.
    pub(crate) fn write(&mut self, address: u16, value: u8, lcd_enabled: bool) -> bool {
        match address {
            0xFF51 => self.source_high = value,
            0xFF52 => self.source_low = value & 0xF0,
            0xFF53 => self.destination_high = value & 0x1F,
            0xFF54 => self.destination_low = value & 0xF0,
            0xFF55 => self.write_control(value, lcd_enabled),
            _ => return false,
        }
        true
    }

    fn write_control(&mut self, value: u8, lcd_enabled: bool) {
        if self.active {
            // Bit 7 clear stops an ongoing HBlank DMA, preserving the
            // remaining block count with inactive bit 7 set.
            if self.mode == TransferMode::HBlank && value & 0x80 == 0 {
                self.stopped_status = 0x80 | (self.blocks_left - 1);
                self.active = false;
                self.block_active = false;
            }
            return;
        }

        self.source = u16::from_be_bytes([self.source_high, self.source_low]);
        self.destination =
            0x8000 | (u16::from(self.destination_high) << 8) | u16::from(self.destination_low);
        self.blocks_left = (value & 0x7F) + 1;
        self.byte_in_block = 0;
        self.phase = 0;
        self.active = true;
        self.mode = if lcd_enabled && value & 0x80 != 0 {
            TransferMode::HBlank
        } else {
            TransferMode::General
        };
        self.block_active = self.mode == TransferMode::General;
        self.stopped_status = 0xFF;
    }

    /// Called at the beginning of the next visible HBlank, not in VBlank.
    pub(crate) fn begin_hblank(&mut self) {
        if self.active && self.mode == TransferMode::HBlank {
            self.block_active = true;
            self.byte_in_block = 0;
            self.phase = 0;
        }
    }

    /// LCD-off HDMA acts as general-purpose transfer in this model.
    pub(crate) fn lcd_disabled(&mut self) {
        if self.active && self.mode == TransferMode::HBlank {
            self.mode = TransferMode::General;
            self.block_active = true;
        }
    }

    /// True when a transfer is currently occupying the CPU.
    pub(crate) fn stalls_cpu(&self) -> bool {
        self.active && self.block_active
    }

    /// True for an active transfer, even while waiting for HBlank.
    pub(crate) fn active(&self) -> bool {
        self.active
    }

    /// Supply the next source/destination address on every second T-cycle.
    /// The bus performs the actual copy, bypassing CPU mode-3 VRAM blocking.
    pub(crate) fn tick_t_cycle(&mut self) -> Option<(u16, u16)> {
        if !self.stalls_cpu() {
            return None;
        }
        self.phase += 1;
        if self.phase != 2 {
            return None;
        }
        self.phase = 0;
        let result = Some((self.source, self.destination));
        self.source = self.source.wrapping_add(1);
        self.destination = self.destination.wrapping_add(1);
        self.byte_in_block += 1;
        if self.byte_in_block == 16 {
            self.byte_in_block = 0;
            self.blocks_left -= 1;
            if self.blocks_left == 0 {
                self.active = false;
                self.block_active = false;
                self.stopped_status = 0xFF;
            } else if self.destination > 0x9FFF {
                // The hardware ends an overflowing destination early;
                // the exact stopped-register status remains unverified.
                self.active = false;
                self.block_active = false;
                self.stopped_status = 0xFF;
            } else if self.mode == TransferMode::HBlank {
                self.block_active = false;
            }
        }
        result
    }
}
