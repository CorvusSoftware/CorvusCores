// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Self-authored MBC3 ROM/RAM banks, RTC selectors and header validation.

use super::{Cartridge, CartridgeError, Mbc3Cartridge};

fn rom(rom_size_code: u8, ram_size_code: u8, cartridge_type: u8) -> Vec<u8> {
    let mut data = vec![0_u8; (2_usize << rom_size_code) * 0x4000];
    for (bank, chunk) in data.chunks_mut(0x4000).enumerate() {
        chunk.fill(bank as u8);
    }
    data[0x0143] = 0x80;
    data[0x0147] = cartridge_type;
    data[0x0148] = rom_size_code;
    data[0x0149] = ram_size_code;
    data
}

fn enable(mapper: &mut Mbc3Cartridge) {
    assert!(mapper.write_rom(0x0000, 0x0A));
}

fn select(mapper: &mut Mbc3Cartridge, selector: u8) {
    assert!(mapper.write_rom(0x4000, selector));
}

fn latch(mapper: &mut Mbc3Cartridge) {
    assert!(mapper.write_rom(0x6000, 0));
    assert!(mapper.write_rom(0x6000, 1));
}

#[test]
fn loader_dispatches_all_standard_mbc3_types_and_rejects_other_mappers() {
    for (kind, ram) in [(0x0F, 0), (0x10, 3), (0x11, 0), (0x12, 2), (0x13, 3)] {
        let cart = Cartridge::load(rom(1, ram, kind)).unwrap();
        assert!(matches!(cart, Cartridge::Mbc3(_)));
        assert_eq!(cart.header().cartridge_type, kind);
    }
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x19)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Mbc3Cartridge::new(rom(1, 0, 0x01)),
        Err(CartridgeError::UnsupportedCartridgeType { code: 1 })
    ));
}

#[test]
fn rom_banking_has_fixed_bank_zero_and_allows_bank_twenty_forty_sixty() {
    let mut mapper = Mbc3Cartridge::new(rom(6, 0, 0x11)).unwrap();
    assert_eq!(mapper.read_rom(0), Some(0));
    assert_eq!(mapper.read_rom(0x3FFF), Some(0));
    assert_eq!(mapper.read_rom(0x4000), Some(1));
    for (selected, expected) in [
        (0, 1),
        (2, 2),
        (0x20, 0x20),
        (0x40, 0x40),
        (0x60, 0x60),
        (0x7F, 0x7F),
        (0x80, 1),
        (0xFF, 0x7F),
    ] {
        assert!(mapper.write_rom(0x2000, selected));
        assert_eq!(mapper.read_rom(0x4000), Some(expected));
        assert_eq!(mapper.read_rom(0), Some(0));
    }
    assert_eq!(mapper.read_rom(0x8000), None);
    assert!(!mapper.write_rom(0x8000, 2));
}

#[test]
fn small_rom_masks_unwired_address_bits_and_zero_aliases_bank_one() {
    let mut mapper = Mbc3Cartridge::new(rom(1, 0, 0x11)).unwrap();
    assert!(mapper.write_rom(0x3000, 0x7F));
    assert_eq!(mapper.read_rom(0x4000), Some(3));
    assert!(mapper.write_rom(0x2000, 0x40));
    assert_eq!(mapper.read_rom(0x4000), Some(0)); // disconnected bank bits
    assert!(mapper.write_rom(0x2000, 0));
    assert_eq!(mapper.read_rom(0x4000), Some(1));
}

#[test]
fn ram_banks_enable_gating_and_invalid_register_selection() {
    let mut mapper = Mbc3Cartridge::new(rom(2, 3, 0x13)).unwrap();
    assert_eq!(mapper.read_ram(0xA000), Some(0xFF));
    assert!(mapper.write_ram(0xA000, 0x90)); // disabled
    assert!(mapper.write_rom(0x1FFF, 0x1A));
    for bank in 0..4 {
        select(&mut mapper, bank);
        assert!(mapper.write_ram(0xA101, 0x40 | bank));
        assert!(mapper.write_ram(0xBFFF, 0x80 | bank));
    }
    for bank in 0..4 {
        select(&mut mapper, bank);
        assert_eq!(mapper.read_ram(0xA101), Some(0x40 | bank));
        assert_eq!(mapper.read_ram(0xBFFF), Some(0x80 | bank));
    }

    select(&mut mapper, 0x04); // invalid selector (not bank 4)
    assert_eq!(mapper.read_ram(0xA000), Some(0xFF));
    assert!(mapper.write_ram(0xA000, 0x11));
    select(&mut mapper, 1);
    assert_eq!(mapper.read_ram(0xA101), Some(0x41));

    assert!(mapper.write_rom(0x0000, 0));
    assert_eq!(mapper.read_ram(0xA101), Some(0xFF));
    assert!(mapper.write_ram(0xA101, 0xAA)); // ignored
    enable(&mut mapper);
    assert_eq!(mapper.read_ram(0xA101), Some(0x41));
    assert_eq!(mapper.read_ram(0x9FFF), None);
    assert!(!mapper.write_ram(0xC000, 0x22));
}

#[test]
fn eight_kib_ram_masks_high_bank_bits() {
    let mut mapper = Mbc3Cartridge::new(rom(1, 2, 0x12)).unwrap();
    enable(&mut mapper);
    select(&mut mapper, 0);
    assert!(mapper.write_ram(0xA000, 0x42));
    select(&mut mapper, 3);
    assert_eq!(mapper.read_ram(0xA000), Some(0x42));
    assert!(mapper.write_ram(0xA000, 0xAB));
    select(&mut mapper, 0);
    assert_eq!(mapper.read_ram(0xA000), Some(0xAB));
}

#[test]
fn rtc_type_zero_f_requires_enable_and_latch_for_reads() {
    let mut mapper = Mbc3Cartridge::new(rom(1, 0, 0x0F)).unwrap();
    select(&mut mapper, 0x08);
    assert!(mapper.write_ram(0xA000, 31)); // ignored while disabled
    mapper.advance_rtc_seconds(65);
    assert_eq!(mapper.read_ram(0xA000), Some(0xFF));

    enable(&mut mapper);
    assert_eq!(mapper.read_ram(0xA000), Some(0)); // initial snapshot
    latch(&mut mapper);
    assert_eq!(mapper.read_ram(0xA000), Some(5));
    select(&mut mapper, 0x09);
    assert_eq!(mapper.read_ram(0xBFFF), Some(1));

    mapper.advance_rtc_seconds(4);
    assert_eq!(mapper.read_ram(0xBFFF), Some(1)); // frozen snapshot
    select(&mut mapper, 0x08);
    assert_eq!(mapper.read_ram(0xA000), Some(5));
    latch(&mut mapper);
    assert_eq!(mapper.read_ram(0xA000), Some(9));

    select(&mut mapper, 0);
    assert_eq!(mapper.read_ram(0xA000), Some(0xFF)); // no SRAM in 0F
}

#[test]
fn rtc_live_writes_are_visible_after_latch_and_share_ram_window() {
    let mut mapper = Mbc3Cartridge::new(rom(1, 3, 0x10)).unwrap();
    enable(&mut mapper);
    select(&mut mapper, 2);
    assert!(mapper.write_ram(0xA000, 0xAC));

    select(&mut mapper, 0x08);
    assert!(mapper.write_ram(0xBFFF, 58));
    select(&mut mapper, 0x09);
    assert!(mapper.write_ram(0xA000, 59));
    select(&mut mapper, 0x0A);
    assert!(mapper.write_ram(0xA000, 23));
    select(&mut mapper, 0x0B);
    assert!(mapper.write_ram(0xA000, 0xFF));
    select(&mut mapper, 0x0C);
    assert!(mapper.write_ram(0xA000, 1));
    latch(&mut mapper);

    assert_eq!(mapper.read_ram(0xBFFF), Some(1));
    mapper.advance_rtc_seconds(2); // day 511 -> 0 and carry
    latch(&mut mapper);
    assert_eq!(mapper.read_ram(0xA000), Some(0x80));
    select(&mut mapper, 0x08);
    assert_eq!(mapper.read_ram(0xA000), Some(0));
    select(&mut mapper, 0);
    assert_eq!(mapper.read_ram(0xA000), Some(0));
    select(&mut mapper, 2);
    assert_eq!(mapper.read_ram(0xA000), Some(0xAC));
}

#[test]
fn variants_without_rtc_ignore_rtc_access_and_elapsed_seconds() {
    for kind in [0x11, 0x12, 0x13] {
        let ram = if kind == 0x11 { 0 } else { 2 };
        let mut mapper = Mbc3Cartridge::new(rom(1, ram, kind)).unwrap();
        enable(&mut mapper);
        select(&mut mapper, 0x08);
        assert!(mapper.write_ram(0xA000, 0xFA));
        mapper.advance_rtc_seconds(987_654);
        latch(&mut mapper);
        assert_eq!(mapper.read_ram(0xA000), Some(0xFF));
    }
}

#[test]
fn invalid_rom_length_mbc30_and_unsupported_ram_sizes_fail() {
    let mut too_short = rom(1, 0, 0x0F);
    too_short.pop();
    assert!(matches!(
        Mbc3Cartridge::new(too_short),
        Err(CartridgeError::IncorrectRomLength {
            expected: 65_536,
            actual: 65_535
        })
    ));

    let mut wrong_rom_size = rom(1, 0, 0x0F);
    wrong_rom_size[0x0148] = 7;
    assert!(matches!(
        Cartridge::load(wrong_rom_size),
        Err(CartridgeError::UnsupportedRomSizeCode { code: 7 })
    ));
    let mut mbc30 = rom(1, 3, 0x10);
    mbc30[0x0149] = 5; // 64-KiB MBC30 SRAM not in standard mapper
    assert!(matches!(
        Cartridge::load(mbc30),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 5 })
    ));
    for (kind, code) in [(0x0F, 2), (0x10, 0), (0x11, 3), (0x12, 1)] {
        assert!(matches!(
            Cartridge::load(rom(1, code, kind)),
            Err(CartridgeError::UnsupportedRamSizeCode { code: found }) if found == code
        ));
    }
}
