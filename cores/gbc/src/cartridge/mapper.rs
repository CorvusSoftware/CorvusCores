// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cartridge-owned mapper dispatch, separate from the system memory bus.
//!
//! The enum makes unsupported hardware explicit: MBC2, MBC5 and other
//! specialty controllers are not silently treated as standard MBC3.

use super::{CartridgeError, CartridgeHeader, Mbc1Cartridge, Mbc3Cartridge, RomOnlyCartridge};

/// A loaded cartridge with its mapper and mutable external RAM state.
#[derive(Debug)]
pub enum Cartridge {
    RomOnly(RomOnlyCartridge),
    Mbc1(Mbc1Cartridge),
    Mbc3(Mbc3Cartridge),
}

impl Cartridge {
    /// Parse a ROM header and construct its supported mapper.
    pub fn load(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        let header = CartridgeHeader::parse(&rom)?;
        match header.cartridge_type {
            0x00 => Ok(Self::RomOnly(RomOnlyCartridge::new(rom)?)),
            0x01..=0x03 => Ok(Self::Mbc1(Mbc1Cartridge::new(rom)?)),
            0x0F..=0x13 => Ok(Self::Mbc3(Mbc3Cartridge::new(rom)?)),
            code => Err(CartridgeError::UnsupportedCartridgeType { code }),
        }
    }

    pub fn header(&self) -> &CartridgeHeader {
        match self {
            Self::RomOnly(cartridge) => cartridge.header(),
            Self::Mbc1(cartridge) => cartridge.header(),
            Self::Mbc3(cartridge) => cartridge.header(),
        }
    }

    pub fn read_rom(&self, address: u16) -> Option<u8> {
        match self {
            Self::RomOnly(cartridge) => cartridge.read_rom(address),
            Self::Mbc1(cartridge) => cartridge.read_rom(address),
            Self::Mbc3(cartridge) => cartridge.read_rom(address),
        }
    }

    pub fn write_rom(&mut self, address: u16, value: u8) -> bool {
        match self {
            Self::RomOnly(_) => address <= 0x7FFF,
            Self::Mbc1(cartridge) => cartridge.write_rom(address, value),
            Self::Mbc3(cartridge) => cartridge.write_rom(address, value),
        }
    }

    /// The cartridge owns the entire A000–BFFF window, including disabled RAM.
    pub fn read_ram(&self, address: u16) -> Option<u8> {
        match self {
            Self::RomOnly(_) => (0xA000..=0xBFFF).contains(&address).then_some(0xFF),
            Self::Mbc1(cartridge) => cartridge.read_ram(address),
            Self::Mbc3(cartridge) => cartridge.read_ram(address),
        }
    }

    pub fn write_ram(&mut self, address: u16, value: u8) -> bool {
        match self {
            Self::RomOnly(_) => (0xA000..=0xBFFF).contains(&address),
            Self::Mbc1(cartridge) => cartridge.write_ram(address, value),
            Self::Mbc3(cartridge) => cartridge.write_ram(address, value),
        }
    }
    /// Advance RTC oscillator time when supplied by the host/frontend.
    /// No effect for cartridge types without an RTC.
    pub fn advance_rtc_seconds(&mut self, seconds: u64) {
        if let Self::Mbc3(cartridge) = self {
            cartridge.advance_rtc_seconds(seconds);
        }
    }
}

impl From<RomOnlyCartridge> for Cartridge {
    fn from(cartridge: RomOnlyCartridge) -> Self {
        Self::RomOnly(cartridge)
    }
}

impl From<Mbc1Cartridge> for Cartridge {
    fn from(cartridge: Mbc1Cartridge) -> Self {
        Self::Mbc1(cartridge)
    }
}

impl From<Mbc3Cartridge> for Cartridge {
    fn from(cartridge: Mbc3Cartridge) -> Self {
        Self::Mbc3(cartridge)
    }
}
