// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Unit tests for cartridge header and ROM-only storage.

use super::header::MIN_HEADER_LENGTH;
use super::rom_only::ROM_ONLY_LENGTH;
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
fn rejects_ram_declared_on_rom_only_cartridge() {
    let mut rom = test_rom(0x80, 0, 0);
    rom[0x0149] = 0x02;
    assert!(matches!(
        RomOnlyCartridge::new(rom),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 0x02 })
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
