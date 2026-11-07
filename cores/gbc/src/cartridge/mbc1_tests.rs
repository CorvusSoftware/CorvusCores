// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic bank-tagged ROMs and volatile RAM for MBC1 regression tests.

use super::{Cartridge, CartridgeError, Mbc1Cartridge, RomOnlyCartridge};

fn rom(rom_size_code: u8, ram_size_code: u8, cartridge_type: u8) -> Vec<u8> {
    let banks = 2_usize << rom_size_code;
    let mut data = vec![0_u8; banks * 0x4000];
    for (bank, chunk) in data.chunks_mut(0x4000).enumerate() {
        chunk.fill(bank as u8);
    }
    data[0x0143] = 0x80;
    data[0x0147] = cartridge_type;
    data[0x0148] = rom_size_code;
    data[0x0149] = ram_size_code;
    data
}

#[test]
fn loader_dispatches_supported_types_and_preserves_rom_only_contracts() {
    assert!(matches!(
        Cartridge::load(rom(0, 0, 0)).unwrap(),
        Cartridge::RomOnly(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 1)).unwrap(),
        Cartridge::Mbc1(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 2, 2)).unwrap(),
        Cartridge::Mbc1(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 3, 3)).unwrap(),
        Cartridge::Mbc1(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x11)),
        Err(CartridgeError::UnsupportedCartridgeType { code: 0x11 })
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x19)),
        Err(CartridgeError::UnsupportedCartridgeType { code: 0x19 })
    ));
    assert!(matches!(
        RomOnlyCartridge::new(rom(1, 0, 1)),
        Err(CartridgeError::UnsupportedCartridgeType { code: 1 })
    ));
}

#[test]
fn default_switchable_bank_is_one_and_zero_aliases_one() {
    let mut cartridge = Mbc1Cartridge::new(rom(2, 0, 1)).unwrap(); // 8 ROM banks
    assert_eq!(cartridge.read_rom(0x0000), Some(0));
    assert_eq!(cartridge.read_rom(0x3FFF), Some(0));
    assert_eq!(cartridge.read_rom(0x4000), Some(1));
    assert_eq!(cartridge.read_rom(0x7FFF), Some(1));

    assert!(cartridge.write_rom(0x2000, 2));
    assert_eq!(cartridge.read_rom(0x4000), Some(2));
    assert!(cartridge.write_rom(0x3FFF, 0));
    assert_eq!(cartridge.read_rom(0x4000), Some(1));
    assert!(cartridge.write_rom(0x2000, 0x20)); // low 5 bits zero
    assert_eq!(cartridge.read_rom(0x4000), Some(1));
    assert!(cartridge.write_rom(0x2000, 0xFF)); // bank 31, mask to 7
    assert_eq!(cartridge.read_rom(0x4000), Some(7));
    assert_eq!(cartridge.read_rom(0x8000), None);
    assert!(!cartridge.write_rom(0x8000, 0));
}

#[test]
fn advanced_mode_selects_lower_rom_window_without_changing_upper_window() {
    let mut cartridge = Mbc1Cartridge::new(rom(6, 0, 1)).unwrap(); // 128 banks
    for upper in 0..4_u8 {
        assert!(cartridge.write_rom(0x4000, upper));
        assert_eq!(cartridge.read_rom(0x0000), Some(0));
        assert_eq!(cartridge.read_rom(0x4000), Some(upper * 32 + 1));
        assert!(cartridge.write_rom(0x6000, 1));
        assert_eq!(cartridge.read_rom(0x0000), Some(upper * 32));
        assert_eq!(cartridge.read_rom(0x4000), Some(upper * 32 + 1));
        assert!(cartridge.write_rom(0x6000, 0));
    }

    assert!(cartridge.write_rom(0x4000, 2));
    assert!(cartridge.write_rom(0x2000, 0x00)); // never bank 40 at 4000
    assert_eq!(cartridge.read_rom(0x4000), Some(65));
    assert!(cartridge.write_rom(0x2000, 31));
    assert_eq!(cartridge.read_rom(0x4000), Some(95));
    assert!(cartridge.write_rom(0x6000, 1));
    assert_eq!(cartridge.read_rom(0x0000), Some(64));
    assert_eq!(cartridge.read_rom(0x4000), Some(95));
}

#[test]
fn unused_high_rom_bits_do_not_select_nonexistent_banks() {
    let mut cartridge = Mbc1Cartridge::new(rom(1, 0, 1)).unwrap(); // 4 banks
    assert!(cartridge.write_rom(0x4000, 3)); // disconnected high bits
    assert!(cartridge.write_rom(0x2000, 2));
    assert_eq!(cartridge.read_rom(0x4000), Some(2));
    assert!(cartridge.write_rom(0x6000, 1));
    assert_eq!(cartridge.read_rom(0x0000), Some(0));
}

#[test]
fn ram_enable_requires_low_nibble_a_and_disabled_writes_are_ignored() {
    let mut cartridge = Mbc1Cartridge::new(rom(1, 2, 2)).unwrap();
    assert_eq!(cartridge.read_ram(0xA000), Some(0xFF));
    assert!(cartridge.write_ram(0xA000, 0x77));
    assert!(cartridge.write_rom(0x0000, 0x1A)); // upper bits irrelevant
    assert_eq!(cartridge.read_ram(0xA000), Some(0));
    assert!(cartridge.write_ram(0xA000, 0x55));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));
    assert!(cartridge.write_rom(0x1FFF, 0x00));
    assert_eq!(cartridge.read_ram(0xA000), Some(0xFF));
    assert!(cartridge.write_ram(0xA000, 0xBB));
    assert!(cartridge.write_rom(0x0000, 0x0A));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));
    assert_eq!(cartridge.read_ram(0x9FFF), None);
    assert!(!cartridge.write_ram(0xC000, 0x77));
}

#[test]
fn ram_bank_mode_selects_four_distinct_banks_and_simple_mode_uses_zero() {
    let mut cartridge = Mbc1Cartridge::new(rom(2, 3, 3)).unwrap();
    assert!(cartridge.write_rom(0x0000, 0x0A)); // enable
    assert!(cartridge.write_rom(0x6000, 1)); // advanced mode
    for bank in 0..4 {
        assert!(cartridge.write_rom(0x4000, bank));
        assert!(cartridge.write_ram(0xA123, bank + 0x40));
        assert!(cartridge.write_ram(0xBFFF, bank + 0x50));
    }
    for bank in 0..4 {
        assert!(cartridge.write_rom(0x4000, bank));
        assert_eq!(cartridge.read_ram(0xA123), Some(bank + 0x40));
        assert_eq!(cartridge.read_ram(0xBFFF), Some(bank + 0x50));
    }
    assert!(cartridge.write_rom(0x6000, 0));
    assert_eq!(cartridge.read_ram(0xA123), Some(0x40));
    assert!(cartridge.write_rom(0x4000, 3)); // high ROM bits do not select RAM
    assert_eq!(cartridge.read_ram(0xA123), Some(0x40));
    assert!(cartridge.write_rom(0x6000, 1));
    assert_eq!(cartridge.read_ram(0xA123), Some(0x43));
}

#[test]
fn eight_kib_ram_ignores_disconnected_ram_bank_bits() {
    let mut cartridge = Mbc1Cartridge::new(rom(0, 2, 2)).unwrap();
    assert!(cartridge.write_rom(0x0000, 0x0A));
    assert!(cartridge.write_rom(0x6000, 1));
    assert!(cartridge.write_rom(0x4000, 0));
    assert!(cartridge.write_ram(0xA000, 0xBE));
    assert!(cartridge.write_rom(0x4000, 3));
    assert_eq!(cartridge.read_ram(0xA000), Some(0xBE));
    assert!(cartridge.write_ram(0xA000, 0xEF));
    assert!(cartridge.write_rom(0x4000, 0));
    assert_eq!(cartridge.read_ram(0xA000), Some(0xEF));
}

#[test]
fn absent_ram_remains_open_when_enable_is_written() {
    let mut cartridge = Mbc1Cartridge::new(rom(1, 0, 1)).unwrap();
    assert!(cartridge.write_rom(0x0000, 0x0A));
    assert!(cartridge.write_ram(0xA000, 0x37));
    assert_eq!(cartridge.read_ram(0xA000), Some(0xFF));
}

#[test]
fn refuses_unsupported_mapper_size_codes_and_incorrect_length() {
    let mut too_short = rom(1, 0, 1);
    too_short.pop();
    assert!(matches!(
        Mbc1Cartridge::new(too_short),
        Err(CartridgeError::IncorrectRomLength {
            expected: 65536,
            actual: 65535
        })
    ));
    let mut bad_rom_size = rom(1, 0, 1);
    bad_rom_size[0x0148] = 0x07; // MBC1 does not wire 4 MiB
    assert!(matches!(
        Cartridge::load(bad_rom_size),
        Err(CartridgeError::UnsupportedRomSizeCode { code: 7 })
    ));
    let mut bad_ram_size = rom(1, 0, 1);
    bad_ram_size[0x0149] = 3; // type 01 has no RAM
    assert!(matches!(
        Cartridge::load(bad_ram_size),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 3 })
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 2)), // MBC1+RAM requires a known size
        Err(CartridgeError::UnsupportedRamSizeCode { code: 0 })
    ));
    let mut tiny_ram = rom(1, 2, 2);
    tiny_ram[0x0149] = 1; // 2 KiB wiring not yet modeled
    assert!(matches!(
        Cartridge::load(tiny_ram),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 1 })
    ));
}

#[test]
fn explicit_cartridge_operations_keep_rom_only_reads_and_writes_no_op() {
    let mut cart = Cartridge::load(rom(0, 0, 0)).unwrap();
    assert!(cart.write_rom(0x2000, 0xFF));
    assert_eq!(cart.read_rom(0x4000), Some(1));
    assert!(cart.write_ram(0xA000, 0x92));
    assert_eq!(cart.read_ram(0xA000), Some(0xFF));
    assert_eq!(cart.read_ram(0xC000), None);
    assert!(!cart.write_ram(0xC000, 0));
}
