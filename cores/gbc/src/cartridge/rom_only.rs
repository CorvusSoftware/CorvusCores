// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Storage for supported 32 KiB ROM-only cartridges.

use super::{CartridgeError, CartridgeHeader};

pub(super) const ROM_ONLY_LENGTH: usize = 0x8000;

/// A 32 KiB cartridge without an MBC or external RAM.
///
/// This direct constructor accepts only type 00. The higher-level
/// `Cartridge::load` dispatches other supported mapper types explicitly.
#[derive(Debug)]
pub struct RomOnlyCartridge {
    rom: Box<[u8]>,
    header: CartridgeHeader,
}

impl RomOnlyCartridge {
    pub fn new(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        let header = CartridgeHeader::parse(&rom)?;

        if header.cartridge_type != 0x00 {
            return Err(CartridgeError::UnsupportedCartridgeType {
                code: header.cartridge_type,
            });
        }
        if header.rom_size_code != 0x00 {
            return Err(CartridgeError::UnsupportedRomSizeCode {
                code: header.rom_size_code,
            });
        }
        if header.ram_size_code != 0x00 {
            return Err(CartridgeError::UnsupportedRamSizeCode {
                code: header.ram_size_code,
            });
        }
        if rom.len() != ROM_ONLY_LENGTH {
            return Err(CartridgeError::IncorrectRomOnlyLength { actual: rom.len() });
        }

        Ok(Self {
            rom: rom.into_boxed_slice(),
            header,
        })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    /// Read the fixed 0x0000–0x7FFF cartridge ROM address range.
    ///
    /// Addresses outside this range are not handled by the cartridge.
    pub fn read_rom(&self, address: u16) -> Option<u8> {
        if usize::from(address) >= ROM_ONLY_LENGTH {
            return None;
        }
        Some(self.rom[usize::from(address)])
    }
}
