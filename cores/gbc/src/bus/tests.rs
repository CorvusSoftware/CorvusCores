// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Existing bus routing, DMG/CGB banking and unmapped-I/O contracts.

use super::{Bus, HardwareMode};
use crate::cartridge::RomOnlyCartridge;

fn test_bus(flag: u8) -> Bus {
    let mut rom = vec![0_u8; 0x8000];
    rom[0] = 0x42;
    rom[0x0143] = flag;
    Bus::new(RomOnlyCartridge::new(rom).unwrap())
}

#[test]
fn routes_rom_without_mutating_rom_only_cartridges() {
    let mut bus = test_bus(0x80);
    assert_eq!(bus.read(0), Some(0x42));
    assert!(bus.write(0, 0x55));
    assert_eq!(bus.read(0), Some(0x42));
    assert_eq!(bus.read(0xA000), Some(0xFF));
    assert!(bus.write(0xA000, 0x33));
    assert_eq!(bus.read(0xA000), Some(0xFF));
}

#[test]
fn routes_vram_wram_echo_oam_and_hram() {
    let mut bus = test_bus(0xC0);
    for (addr, value) in [
        (0x8000, 1),
        (0x9FFF, 2),
        (0xC000, 3),
        (0xD000, 4),
        (0xFE00, 5),
        (0xFE9F, 6),
        (0xFF80, 7),
        (0xFFFE, 8),
    ] {
        assert!(bus.write(addr, value));
        assert_eq!(bus.read(addr), Some(value));
    }
    assert_eq!(bus.read(0xE000), Some(3));
    assert_eq!(bus.read(0xF000), Some(4));
    assert!(bus.write(0xFDFF, 9));
    assert_eq!(bus.read(0xDDFF), Some(9));
    assert_eq!(bus.read(0xFEA0), None);
    assert!(!bus.write(0xFEA0, 1));
}

#[test]
fn banking_registers_route_to_their_own_components() {
    let mut bus = test_bus(0x80);
    assert_eq!(bus.mode(), HardwareMode::Cgb);
    assert!(bus.write(0x8000, 0x10));
    assert!(bus.write(0xFF4F, 0x01));
    assert_eq!(bus.read(0xFF4F), Some(0xFF));
    assert!(bus.write(0x8000, 0x20));
    assert!(bus.write(0xFF4F, 0));
    assert_eq!(bus.read(0x8000), Some(0x10));
    assert!(bus.write(0xFF4F, 1));
    assert_eq!(bus.read(0x8000), Some(0x20));

    assert!(bus.write(0xD000, 0x40));
    assert!(bus.write(0xFF70, 3));
    assert!(bus.write(0xD000, 0x60));
    assert_eq!(bus.read(0xFF70), Some(0xFB));
    assert!(bus.write(0xFF70, 0));
    assert_eq!(bus.read(0xD000), Some(0x40));
    assert_eq!(bus.read(0xFF70), Some(0xF8));
    assert!(bus.write(0xFF70, 3));
    assert_eq!(bus.read(0xD000), Some(0x60));
}

#[test]
fn original_game_boy_titles_use_compatibility_banking() {
    let mut bus = test_bus(0x00);
    assert_eq!(bus.mode(), HardwareMode::DmgCompatibility);
    assert!(!bus.write(0xFF4F, 1));
    assert!(!bus.write(0xFF70, 3));
    assert_eq!(bus.read(0xFF4F), None);
    assert_eq!(bus.read(0xFF70), None);
    assert!(bus.write(0xD000, 0x77));
    assert_eq!(bus.read(0xD000), Some(0x77));
}

#[test]
fn unsupported_registers_are_not_silently_fake_mapped() {
    let mut bus = test_bus(0x80);
    // FF00 is now joypad, so exclude it from unmapped addresses
    for addr in [0xFEA0, 0xFF01] {
        assert_eq!(bus.read(addr), None);
        assert!(!bus.write(addr, 0xAA));
    }
    // FF00 is joypad - should be readable and writable
    assert_eq!(bus.read(0xFF00), Some(0xFF));
    assert!(bus.write(0xFF00, 0xAA));
}
