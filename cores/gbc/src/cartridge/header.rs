//! Game Boy / Game Boy Color cartridge header parsing.
//!
//! Hardware reference: <https://gbdev.io/pandocs/The_Cartridge_Header.html>

use std::fmt;

pub(super) const MIN_HEADER_LENGTH: usize = 0x150;

/// Compatibility advertised by a cartridge header, not the current hardware mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CgbSupport {
    /// A Game Boy title; a CGB runs it in non-CGB compatibility mode.
    DmgOnly,
    /// A color-enhanced title that also runs on an original Game Boy.
    Enhanced,
    /// A title whose header marks it as CGB-only.
    Exclusive,
}

/// Header fields needed for the first cartridge loading milestone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CartridgeHeader {
    pub cgb_support: CgbSupport,
    pub cartridge_type: u8,
    pub rom_size_code: u8,
    pub ram_size_code: u8,
    /// The checksum at 0x014D matches the bytes at 0x0134 through 0x014C.
    pub header_checksum_valid: bool,
}

impl CartridgeHeader {
    /// Inspect a cartridge header without assuming support for its mapper.
    ///
    /// A checksum mismatch is reported, not rejected: enforcing boot-ROM
    /// behavior is the responsibility of later hardware emulation.
    pub fn parse(rom: &[u8]) -> Result<Self, CartridgeError> {
        if rom.len() < MIN_HEADER_LENGTH {
            return Err(CartridgeError::HeaderTooShort { actual: rom.len() });
        }

        let cgb_flag = rom[0x0143];
        let cgb_support = if cgb_flag & 0x80 == 0 {
            CgbSupport::DmgOnly
        } else if cgb_flag & 0x40 == 0 {
            CgbSupport::Enhanced
        } else {
            CgbSupport::Exclusive
        };

        let checksum = (0x0134..=0x014C).fold(0_u8, |sum, address| {
            sum.wrapping_sub(rom[address]).wrapping_sub(1)
        });

        Ok(Self {
            cgb_support,
            cartridge_type: rom[0x0147],
            rom_size_code: rom[0x0148],
            ram_size_code: rom[0x0149],
            header_checksum_valid: checksum == rom[0x014D],
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartridgeError {
    HeaderTooShort { actual: usize },
    UnsupportedCartridgeType { code: u8 },
    UnsupportedRomSizeCode { code: u8 },
    IncorrectRomOnlyLength { actual: usize },
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual } => {
                write!(f, "cartridge header is too short: {actual} bytes")
            }
            Self::UnsupportedCartridgeType { code } => {
                write!(f, "cartridge type 0x{code:02X} is not supported yet")
            }
            Self::UnsupportedRomSizeCode { code } => {
                write!(f, "ROM size code 0x{code:02X} is not supported by ROM-only cartridges")
            }
            Self::IncorrectRomOnlyLength { actual } => {
                write!(f, "expected a 32768-byte ROM-only cartridge, got {actual} bytes")
            }
        }
    }
}

impl std::error::Error for CartridgeError {}

