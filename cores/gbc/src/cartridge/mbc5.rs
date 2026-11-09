// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Standard MBC5 ROM/RAM banking for Game Boy and Game Boy Color.
//!
//! Supports cartridge types 19-1E with up to 512 ROM banks (64 MiB)
//! and up to 16 RAM banks (128 KiB). Battery-backed persistence and
//! electrical open-bus values are intentionally not modeled. RAM is
//! volatile and initialized to zero.
//! See https://gbdev.io/pandocs/MBC5.html.

use super::{CartridgeError, CartridgeHeader};

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

/// Owns ROM bytes, optional external RAM and the MBC5 control latches.
#[derive(Debug)]
pub struct Mbc5Cartridge {
    rom: Box<[u8]>,
    ram: Box<[u8]>,
    header: CartridgeHeader,
    rom_bank_mask: usize,
    ram_bank_mask: usize,
    ram_enabled: bool,
    rom_bank_low: u8,
    rom_bank_high: u8,
    ram_bank: u8,
}

impl Mbc5Cartridge {
    pub fn new(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        let header = CartridgeHeader::parse(&rom)?;
        if !(0x19..=0x1E).contains(&header.cartridge_type) {
            return Err(CartridgeError::UnsupportedCartridgeType {
                code: header.cartridge_type,
            });
        }

        // MBC5 has up to nine ROM bank address bits (512 banks = 64 MiB).
        // All supported bank counts are powers of two.
        let rom_banks = match header.rom_size_code {
            0x00..=0x08 => 2_usize << header.rom_size_code,
            code => return Err(CartridgeError::UnsupportedRomSizeCode { code }),
        };
        let expected = rom_banks * ROM_BANK_SIZE;
        if rom.len() != expected {
            return Err(CartridgeError::IncorrectRomLength {
                expected,
                actual: rom.len(),
            });
        }

        // MBC5 supports up to 16 RAM banks (128 KiB).
        // Types 19/1C have no RAM, 1A/1D have 8 KiB (1 bank), 1B/1E have 128 KiB (16 banks).
        let ram_bytes = match (header.cartridge_type, header.ram_size_code) {
            (0x19 | 0x1C, 0x00) => 0,
            (0x1A | 0x1D, 0x00) => 0,
            (0x1A | 0x1D, 0x02) => RAM_BANK_SIZE,
            (0x1B | 0x1E, 0x00) => 0,
            (0x1B | 0x1E, 0x05) => 16 * RAM_BANK_SIZE,
            (_, code) => return Err(CartridgeError::UnsupportedRamSizeCode { code }),
        };

        Ok(Self {
            rom: rom.into_boxed_slice(),
            ram: vec![0; ram_bytes].into_boxed_slice(),
            header,
            rom_bank_mask: rom_banks - 1,
            ram_bank_mask: (ram_bytes / RAM_BANK_SIZE).saturating_sub(1),
            ram_enabled: false,
            rom_bank_low: 1,
            rom_bank_high: 0,
            ram_bank: 0,
        })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    /// Bank zero is fixed at 0000-3FFF; selected bank combines low and high bits.
    /// Unlike MBC1, MBC5 does NOT alias bank 0 to bank 1 in the switchable area.
    pub fn read_rom(&self, address: u16) -> Option<u8> {
        let (bank, offset) = match address {
            0x0000..=0x3FFF => (0, usize::from(address)),
            0x4000..=0x7FFF => {
                let bank = (usize::from(self.rom_bank_high) << 8) | usize::from(self.rom_bank_low);
                (bank & self.rom_bank_mask, usize::from(address - 0x4000))
            }
            _ => return None,
        };
        Some(self.rom[bank * ROM_BANK_SIZE + offset])
    }

    /// Writes update RAM enable, ROM bank (low/high) or RAM bank selection.
    pub fn write_rom(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,
            0x2000..=0x2FFF => self.rom_bank_low = value,
            0x3000..=0x3FFF => self.rom_bank_high = value & 0x01,
            0x4000..=0x5FFF => self.ram_bank = value & 0x0F,
            _ => return false,
        }
        true
    }

    fn ram_offset(&self, address: u16) -> Option<usize> {
        if !(0xA000..=0xBFFF).contains(&address) {
            return None;
        }
        let bank = usize::from(self.ram_bank) & self.ram_bank_mask;
        Some(bank * RAM_BANK_SIZE + usize::from(address - 0xA000))
    }

    /// A disabled, absent or unaddressable RAM byte reads as FF.
    pub fn read_ram(&self, address: u16) -> Option<u8> {
        let index = self.ram_offset(address)?;
        Some(if self.ram_enabled {
            self.ram.get(index).copied().unwrap_or(0xFF)
        } else {
            0xFF
        })
    }

    /// RAM-disabled writes are valid no-ops, not writes to unsupported I/O.
    pub fn write_ram(&mut self, address: u16, value: u8) -> bool {
        let Some(index) = self.ram_offset(address) else {
            return false;
        };
        if self.ram_enabled {
            if let Some(byte) = self.ram.get_mut(index) {
                *byte = value;
            }
        }
        true
    }
}
