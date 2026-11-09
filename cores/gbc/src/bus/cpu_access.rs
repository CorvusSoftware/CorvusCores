// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! CPU-visible memory map and LCD/DMA access restrictions.
//!
//! This is distinct from DMA source reads in `clock`: CPU reads may be
//! blocked by PPU modes while internal DMA transfers still access VRAM.

use super::{Bus, HardwareMode};

impl Bus {
    /// Read an implemented address. `None` means not yet modeled, not
    /// necessarily a specific value returned by real hardware.
    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x0000..=0x7FFF => self.cartridge.read_rom(address),
            0x8000..=0x9FFF => {
                if self.ppu.cpu_can_access_vram() {
                    self.vram.read(address)
                } else {
                    Some(0xFF)
                }
            }
            0xA000..=0xBFFF => self.cartridge.read_ram(address),
            0xC000..=0xFDFF => self.wram.read(address),
            0xFE00..=0xFE9F => {
                if self.ppu.cpu_can_access_oam() && !self.dma.active() {
                    self.oam.read(address)
                } else {
                    Some(0xFF)
                }
            }
            0xFF00 => Some(self.joypad.read()),
            0xFF04..=0xFF07 => self.timer.read(address),
            0xFF0F => Some(self.interrupts.read_if()),
            0xFF40..=0xFF45 | 0xFF47..=0xFF4B => self.ppu.read(address),
            0xFF46 => Some(self.dma.source_high()),
            0xFF4F => self.vram.read_bank_register(),
            0xFF51..=0xFF55 if self.mode == HardwareMode::Cgb => self.vram_dma.read(address),
            0xFF68..=0xFF6B => self.ppu.read(address),
            0xFF70 => self.wram.read_bank_register(),
            0xFF80..=0xFFFE => self.hram.read(address),
            0xFFFF => Some(self.interrupts.read_ie()),
            _ => None,
        }
    }

    /// Write to an implemented address. `false` means the target is
    /// unsupported; writes to ROM or absent cartridge RAM are valid no-ops.
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x7FFF => self.cartridge.write_rom(address, value),
            0xA000..=0xBFFF => self.cartridge.write_ram(address, value),
            0x8000..=0x9FFF => {
                if self.ppu.cpu_can_access_vram() {
                    self.vram.write(address, value)
                } else {
                    true // CPU write ignored, not an unsupported address.
                }
            }
            0xC000..=0xFDFF => self.wram.write(address, value),
            0xFE00..=0xFE9F => {
                if self.ppu.cpu_can_access_oam() && !self.dma.active() {
                    self.oam.write(address, value)
                } else {
                    true // CPU write ignored while PPU or DMA owns OAM.
                }
            }
            0xFF00 => {
                self.joypad.write(value);
                true
            }
            0xFF04..=0xFF07 => self.timer.write(address, value),
            0xFF0F => {
                self.interrupts.write_if(value);
                true
            }
            0xFF40..=0xFF45 | 0xFF47..=0xFF4B => {
                let written = self.ppu.write(address, value, &mut self.interrupts);
                if address == 0xFF40 && !self.ppu.lcd_enabled() {
                    self.vram_dma.lcd_disabled();
                }
                written
            }
            0xFF46 => {
                self.dma.start(value);
                true
            }
            0xFF4F => self.vram.write_bank_register(value),
            0xFF51..=0xFF55 if self.mode == HardwareMode::Cgb => {
                self.vram_dma.write(address, value, self.ppu.lcd_enabled())
            }
            0xFF68..=0xFF6B => self.ppu.write(address, value, &mut self.interrupts),
            0xFF70 => self.wram.write_bank_register(value),
            0xFF80..=0xFFFE => self.hram.write(address, value),
            0xFFFF => {
                self.interrupts.write_ie(value);
                true
            }
            _ => false,
        }
    }
}
