// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Standard MBC3 ROM/RAM banking and optional real-time clock.
//!
//! Types 0F/10 provide an RTC; types 11/12/13 do not. ROM size is limited
//! to 2 MiB and RAM to 32 KiB. The MBC30 variant (64 KiB RAM), battery
//! persistence and host-time synchronization are not implemented.
//! See https://gbdev.io/pandocs/MBC3.html.

use super::rtc::Rtc;
use super::{CartridgeError, CartridgeHeader};

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

/// Mapper registers, ROM/RAM storage and optional MBC3 RTC.
#[derive(Debug)]
pub struct Mbc3Cartridge {
    rom: Box<[u8]>,
    ram: Box<[u8]>,
    header: CartridgeHeader,
    rom_bank_mask: usize,
    ram_bank_mask: usize,
    ram_rtc_enabled: bool,
    rom_bank: u8,
    ram_rtc_select: u8,
    rtc: Option<Rtc>,
}

impl Mbc3Cartridge {
    pub fn new(rom: Vec<u8>) -> Result<Self, CartridgeError> {
        let header = CartridgeHeader::parse(&rom)?;
        if !(0x0F..=0x13).contains(&header.cartridge_type) {
            return Err(CartridgeError::UnsupportedCartridgeType {
                code: header.cartridge_type,
            });
        }

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

        // A RAM-less RTC cartridge is type 0F. MBC3+RAM is either one
        // 8-KiB bank or four 8-KiB banks. MBC30's 64-KiB layout is excluded.
        let ram_bytes = match (header.cartridge_type, header.ram_size_code) {
            (0x0F | 0x11, 0x00) => 0,
            (0x10 | 0x12 | 0x13, 0x02) => RAM_BANK_SIZE,
            (0x10 | 0x12 | 0x13, 0x03) => 4 * RAM_BANK_SIZE,
            (_, code) => return Err(CartridgeError::UnsupportedRamSizeCode { code }),
        };

        let has_rtc = matches!(header.cartridge_type, 0x0F | 0x10);
        Ok(Self {
            rom: rom.into_boxed_slice(),
            ram: vec![0; ram_bytes].into_boxed_slice(),
            header,
            rom_bank_mask: rom_banks - 1,
            ram_bank_mask: (ram_bytes / RAM_BANK_SIZE).saturating_sub(1),
            ram_rtc_enabled: false,
            rom_bank: 1,
            ram_rtc_select: 0,
            rtc: has_rtc.then(Rtc::new),
        })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    /// Bank zero is fixed at 0000–3FFF; a selected zero maps bank one.
    pub fn read_rom(&self, address: u16) -> Option<u8> {
        let (bank, offset) = match address {
            0x0000..=0x3FFF => (0, usize::from(address)),
            0x4000..=0x7FFF => (
                usize::from(self.rom_bank) & self.rom_bank_mask,
                usize::from(address - 0x4000),
            ),
            _ => return None,
        };
        Some(self.rom[bank * ROM_BANK_SIZE + offset])
    }

    /// Writes update RAM/RTC enable, ROM bank, RAM/RTC select or latch.
    pub fn write_rom(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x1FFF => self.ram_rtc_enabled = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank };
            }
            0x4000..=0x5FFF => self.ram_rtc_select = value,
            0x6000..=0x7FFF => {
                if let Some(rtc) = &mut self.rtc {
                    rtc.write_latch(value);
                }
            }
            _ => return false,
        }
        true
    }

    /// Disabled RAM/RTC and unavailable selectors read deterministic FF.
    pub fn read_ram(&self, address: u16) -> Option<u8> {
        if !(0xA000..=0xBFFF).contains(&address) {
            return None;
        }
        if !self.ram_rtc_enabled {
            return Some(0xFF);
        }
        let value = match self.ram_rtc_select {
            bank @ 0..=3 => {
                let index = (usize::from(bank) & self.ram_bank_mask) * RAM_BANK_SIZE
                    + usize::from(address - 0xA000);
                self.ram.get(index).copied().unwrap_or(0xFF)
            }
            register @ 0x08..=0x0C => self
                .rtc
                .as_ref()
                .map_or(0xFF, |rtc| rtc.read(register)),
            _ => 0xFF,
        };
        Some(value)
    }

    /// CPU writes to disabled/absent RAM or RTC are valid no-ops.
    pub fn write_ram(&mut self, address: u16, value: u8) -> bool {
        if !(0xA000..=0xBFFF).contains(&address) {
            return false;
        }
        if !self.ram_rtc_enabled {
            return true;
        }
        match self.ram_rtc_select {
            bank @ 0..=3 => {
                let index = (usize::from(bank) & self.ram_bank_mask) * RAM_BANK_SIZE
                    + usize::from(address - 0xA000);
                if let Some(byte) = self.ram.get_mut(index) {
                    *byte = value;
                }
            }
            register @ 0x08..=0x0C => {
                if let Some(rtc) = &mut self.rtc {
                    rtc.write(register, value);
                }
            }
            _ => {}
        }
        true
    }

    /// Advance the independent RTC oscillator using explicitly supplied time.
    ///
    /// Does not access the host clock and does not derive time from CPU T-cycles.
    /// No-ops on MBC3 variants without an RTC.
    pub fn advance_rtc_seconds(&mut self, seconds: u64) {
        if let Some(rtc) = &mut self.rtc {
            rtc.advance_seconds(seconds);
        }
    }
}
