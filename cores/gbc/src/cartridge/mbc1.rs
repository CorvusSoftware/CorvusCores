// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Standard MBC1 ROM/RAM banking for Game Boy and Game Boy Color.
//!
//! Supports cartridge types 01–03 and power-of-two ROM images from 32 KiB
//! through 2 MiB, with no RAM, 8 KiB or 32 KiB external RAM. MBC1M multicart
//! wiring, 2 KiB RAM, persistent battery saves and electrical open-bus values
//! are intentionally not modeled. RAM is volatile and initialized to zero.
//! See https://gbdev.io/pandocs/MBC1.html.

use super::{CartridgeError, CartridgeHeader};

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

/// Owns ROM bytes, optional external RAM and the four MBC1 control latches.
#[derive(Debug)]
pub struct Mbc1Cartridge {
    rom: Box<[u8]>,
    ram: Box<[u8]>,
    header: CartridgeHeader,
    rom_bank_mask: usize,
    ram_enabled: bool,
    low_rom_bank: u8,
    upper_bank: u8,
    advanced_mode: bool,
}

impl Mbc1Cartridge {
    pub fn new(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        let header = CartridgeHeader::parse(&rom)?;
        if !(0x01..=0x03).contains(&header.cartridge_type) {
            return Err(CartridgeError::UnsupportedCartridgeType {
                code: header.cartridge_type,
            });
        }

        // MBC1 has at most seven ROM bank address bits. All supported bank
        // counts are powers of two, so disconnected upper bits can be masked.
        let rom_banks = match header.rom_size_code {
            0x00..=0x06 => 2_usize << header.rom_size_code,
            code => return Err(CartridgeError::UnsupportedRomSizeCode { code }),
        };
        let expected = rom_banks * ROM_BANK_SIZE;
        if rom.len() != expected {
            return Err(CartridgeError::IncorrectRomLength {
                expected,
                actual: rom.len(),
            });
        }

        let ram_bytes = match (header.cartridge_type, header.ram_size_code) {
            (0x01, 0x00) => 0,
            (0x02 | 0x03, 0x02) => RAM_BANK_SIZE,
            (0x02 | 0x03, 0x03) => 4 * RAM_BANK_SIZE,
            (_, code) => return Err(CartridgeError::UnsupportedRamSizeCode { code }),
        };

        Ok(Self {
            rom: rom.into_boxed_slice(),
            ram: vec![0; ram_bytes].into_boxed_slice(),
            header,
            rom_bank_mask: rom_banks - 1,
            ram_enabled: false,
            low_rom_bank: 1,
            upper_bank: 0,
            advanced_mode: false,
        })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    /// The lower window selects bank 00 or banks 20/40/60 in advanced mode.
    /// The upper window always combines the five low and two high ROM bits;
    /// a low-bank register value of zero aliases to one.
    pub fn read_rom(&self, address: u16) -> Option<u8> {
        let (bank, offset) = match address {
            0x0000..=0x3FFF => (
                if self.advanced_mode {
                    usize::from(self.upper_bank) << 5
                } else {
                    0
                },
                usize::from(address),
            ),
            0x4000..=0x7FFF => (
                (usize::from(self.upper_bank) << 5) | usize::from(self.low_rom_bank),
                usize::from(address - 0x4000),
            ),
            _ => return None,
        };
        let bank = bank & self.rom_bank_mask;
        Some(self.rom[bank * ROM_BANK_SIZE + offset])
    }

    /// Writes in 0000–7FFF change the MBC1 latches; ROM remains immutable.
    pub fn write_rom(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => {
                let bank = value & 0x1F;
                self.low_rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => self.upper_bank = value & 0x03,
            0x6000..=0x7FFF => self.advanced_mode = value & 1 != 0,
            _ => return false,
        }
        true
    }

    fn ram_offset(&self, address: u16) -> Option<usize> {
        if !(0xA000..=0xBFFF).contains(&address) {
            return None;
        }
        let bank = if self.advanced_mode {
            usize::from(self.upper_bank)
        } else {
            0
        };
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
