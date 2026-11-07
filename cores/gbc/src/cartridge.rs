//! Game Boy / Game Boy Color cartridge header parsing and ROM-only storage.
//!
//! Hardware reference: <https://gbdev.io/pandocs/The_Cartridge_Header.html>

use std::fmt;

const MIN_HEADER_LENGTH: usize = 0x150;
const ROM_ONLY_LENGTH: usize = 0x8000;

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

/// A 32 KiB cartridge without an MBC or external RAM.
///
/// Other cartridge types are rejected explicitly until their own mappers
/// are implemented; the ROM bytes are not silently treated as ROM-only.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rom(cgb_flag: u8, cartridge_type: u8, rom_size_code: u8) -> Vec<u8> {
        let mut rom = vec![0_u8; ROM_ONLY_LENGTH];
        rom[0x0143] = cgb_flag;
        rom[0x0147] = cartridge_type;
        rom[0x0148] = rom_size_code;
        let checksum = (0x0134..=0x014C).fold(0_u8, |sum, address| {
            sum.wrapping_sub(rom[address]).wrapping_sub(1)
        });
        rom[0x014D] = checksum;
        rom
    }

    #[test]
    fn distinguishes_dmg_enhanced_and_exclusive_headers() {
        for (flag, expected) in [
            (0x00, CgbSupport::DmgOnly),
            (0x80, CgbSupport::Enhanced),
            (0xC0, CgbSupport::Exclusive),
        ] {
            let header = CartridgeHeader::parse(&test_rom(flag, 0, 0)).unwrap();
            assert_eq!(header.cgb_support, expected);
            assert!(header.header_checksum_valid);
        }
    }

    #[test]
    fn rejects_a_truncated_header() {
        let rom = vec![0_u8; MIN_HEADER_LENGTH - 1];
        assert_eq!(
            CartridgeHeader::parse(&rom),
            Err(CartridgeError::HeaderTooShort { actual: rom.len() })
        );
    }

    #[test]
    fn reports_a_bad_checksum_without_rejecting_the_header() {
        let mut rom = test_rom(0x80, 0, 0);
        rom[0x0134] ^= 0x01;
        let header = CartridgeHeader::parse(&rom).unwrap();
        assert!(!header.header_checksum_valid);
    }

    #[test]
    fn reads_both_fixed_rom_banks() {
        let mut rom = test_rom(0x80, 0, 0);
        rom[0x0000] = 0x12;
        rom[0x4000] = 0x34;
        rom[0x7FFF] = 0x56;
        let cartridge = RomOnlyCartridge::new(rom).unwrap();

        assert_eq!(cartridge.header().cgb_support, CgbSupport::Enhanced);
        assert_eq!(cartridge.read_rom(0x0000), Some(0x12));
        assert_eq!(cartridge.read_rom(0x4000), Some(0x34));
        assert_eq!(cartridge.read_rom(0x7FFF), Some(0x56));
        assert_eq!(cartridge.read_rom(0x8000), None);
    }

    #[test]
    fn does_not_silently_treat_an_mbc1_cart_as_rom_only() {
        let rom = test_rom(0x80, 0x01, 0);
        assert!(matches!(
            RomOnlyCartridge::new(rom),
            Err(CartridgeError::UnsupportedCartridgeType { code: 0x01 })
        ));
    }

    #[test]
    fn rejects_an_inconsistent_rom_size_code() {
        let rom = test_rom(0x80, 0x00, 0x01);
        assert!(matches!(
            RomOnlyCartridge::new(rom),
            Err(CartridgeError::UnsupportedRomSizeCode { code: 0x01 })
        ));
    }

    #[test]
    fn rejects_an_incorrect_rom_only_length() {
        let mut rom = test_rom(0x80, 0, 0);
        rom.push(0);
        assert!(matches!(
            RomOnlyCartridge::new(rom),
            Err(CartridgeError::IncorrectRomOnlyLength { actual: 32769 })
        ));
    }
}
