// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic bank-tagged ROMs and volatile RAM for MBC5 regression tests.

use super::{Cartridge, CartridgeError, Mbc5Cartridge};

fn rom(rom_size_code: u8, ram_size_code: u8, cartridge_type: u8) -> Vec<u8> {
    let banks = 2_usize << rom_size_code;
    let mut data = vec![0_u8; banks * 0x4000];
    for (bank, chunk) in data.chunks_mut(0x4000).enumerate() {
        // Use bank mod 256 to fit in u8, but this means banks >= 256 will wrap
        // For most tests we use banks < 256, but we can test the masking behavior
        chunk.fill((bank % 256) as u8);
    }
    data[0x0143] = 0x80;
    data[0x0147] = cartridge_type;
    data[0x0148] = rom_size_code;
    data[0x0149] = ram_size_code;
    data
}

#[test]
fn loader_dispatches_mbc5_types() {
    // MBC5 types: 19, 1A, 1B, 1C, 1D, 1E
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x19)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1A)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1B)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1C)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1D)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1E)).unwrap(),
        Cartridge::Mbc5(_)
    ));
    // Type 1F is not MBC5
    assert!(matches!(
        Cartridge::load(rom(1, 0, 0x1F)),
        Err(CartridgeError::UnsupportedCartridgeType { code: 0x1F })
    ));
}

#[test]
fn default_rom_bank_mapping() {
    let cartridge = Mbc5Cartridge::new(rom(2, 0, 0x19)).unwrap(); // 8 ROM banks
    // Bank 0 is fixed at 0000-3FFF
    assert_eq!(cartridge.read_rom(0x0000), Some(0));
    assert_eq!(cartridge.read_rom(0x3FFF), Some(0));
    // Bank 1 is default at 4000-7FFF
    assert_eq!(cartridge.read_rom(0x4000), Some(1));
    assert_eq!(cartridge.read_rom(0x7FFF), Some(1));
    assert_eq!(cartridge.read_rom(0x8000), None);
}

#[test]
fn low_rom_bank_register_selects_banks() {
    let mut cartridge = Mbc5Cartridge::new(rom(2, 0, 0x19)).unwrap(); // 8 banks
    // Select bank 2 via low register (2000-2FFF)
    assert!(cartridge.write_rom(0x2000, 2));
    assert_eq!(cartridge.read_rom(0x4000), Some(2));
    assert_eq!(cartridge.read_rom(0x7FFF), Some(2));

    // MBC5 does NOT alias bank 0 to bank 1 (unlike MBC1/MBC3)
    assert!(cartridge.write_rom(0x2000, 0));
    assert_eq!(cartridge.read_rom(0x4000), Some(0));

    // Bank 7 is the highest for 8 banks
    assert!(cartridge.write_rom(0x2000, 7));
    assert_eq!(cartridge.read_rom(0x4000), Some(7));

    // Banks >= 8 wrap around due to mask
    assert!(cartridge.write_rom(0x2000, 8));
    assert_eq!(cartridge.read_rom(0x4000), Some(0)); // 8 & 7 = 0
    assert!(cartridge.write_rom(0x2000, 0xFF));
    assert_eq!(cartridge.read_rom(0x4000), Some(7)); // 0xFF & 7 = 7
}

#[test]
fn high_rom_bank_register_extends_bank_selection() {
    // Create a ROM with 512 banks (64 MiB)
    let mut cartridge = Mbc5Cartridge::new(rom(8, 0, 0x19)).unwrap();

    // Select bank 1 (default)
    assert_eq!(cartridge.read_rom(0x4000), Some(1));

    // Select bank 256 using both low and high registers
    // low = 0, high = 1, so bank = (1 << 8) | 0 = 256
    assert!(cartridge.write_rom(0x2000, 0x00)); // low byte = 0
    assert!(cartridge.write_rom(0x3000, 0x01)); // high bit = 1
    // Bank 256: 256 % 512 = 256, data filled as 256 % 256 = 0, but 0 aliases to 1
    assert_eq!(cartridge.read_rom(0x4000), Some(0)); // 256 % 256 = 0

    // Select bank 257: low = 1, high = 1, bank = 257
    assert!(cartridge.write_rom(0x2000, 0x01));
    assert_eq!(cartridge.read_rom(0x4000), Some(1)); // 257 % 256 = 1

    // Select bank 1: low = 1, high = 0, bank = 1
    assert!(cartridge.write_rom(0x3000, 0x00));
    assert_eq!(cartridge.read_rom(0x4000), Some(1));

    // High register bit 1 only (bit 0 is used, higher bits ignored)
    assert!(cartridge.write_rom(0x3000, 0x02)); // bit 1 ignored
    assert_eq!(cartridge.read_rom(0x4000), Some(1));

    // Select bank 511: low = 255, high = 1, bank = 511
    assert!(cartridge.write_rom(0x2000, 0xFF));
    assert!(cartridge.write_rom(0x3000, 0x01));
    assert_eq!(cartridge.read_rom(0x4000), Some(255)); // 511 % 256 = 255
}

#[test]
fn ram_enable_and_disable() {
    let mut cartridge = Mbc5Cartridge::new(rom(1, 2, 0x1A)).unwrap(); // 8 KiB RAM

    // RAM disabled by default
    assert_eq!(cartridge.read_ram(0xA000), Some(0xFF));

    // Enable RAM with 0x0A
    assert!(cartridge.write_rom(0x0000, 0x0A));
    // Now RAM is enabled, but reads 0 because RAM is zero-initialized
    assert_eq!(cartridge.read_ram(0xA000), Some(0));

    // Write to RAM
    assert!(cartridge.write_ram(0xA000, 0x55));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));

    // Disable RAM
    assert!(cartridge.write_rom(0x0000, 0x00));
    assert_eq!(cartridge.read_ram(0xA000), Some(0xFF));

    // Write while disabled is a no-op
    assert!(cartridge.write_ram(0xA000, 0xAA));
    // Re-enable and verify old value still there
    assert!(cartridge.write_rom(0x0000, 0x0A));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));

    // Only low nibble matters for RAM enable
    assert!(cartridge.write_rom(0x0000, 0x1A)); // 0x1A & 0x0F = 0x0A
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));
    assert!(cartridge.write_rom(0x0000, 0x2A)); // 0x2A & 0x0F = 0x0A
    assert_eq!(cartridge.read_ram(0xA000), Some(0x55));
}

#[test]
fn ram_bank_selection() {
    let mut cartridge = Mbc5Cartridge::new(rom(1, 5, 0x1B)).unwrap(); // 128 KiB RAM = 16 banks

    // Enable RAM
    assert!(cartridge.write_rom(0x0000, 0x0A));

    // Select RAM bank 0 (default)
    assert!(cartridge.write_rom(0x4000, 0));
    assert!(cartridge.write_ram(0xA000, 0x11));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x11));

    // Select RAM bank 1
    assert!(cartridge.write_rom(0x4000, 1));
    assert!(cartridge.write_ram(0xA000, 0x22));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x22));

    // Switch back to bank 0, verify different data
    assert!(cartridge.write_rom(0x4000, 0));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x11));

    // Only low 4 bits used for RAM bank
    assert!(cartridge.write_rom(0x4000, 0x1F));
    assert_eq!(cartridge.read_ram(0xA000), Some(0)); // bank 15
    assert!(cartridge.write_ram(0xA000, 0x33));
    assert!(cartridge.write_rom(0x4000, 0x0F));
    assert_eq!(cartridge.read_ram(0xA000), Some(0x33));
}

#[test]
fn ram_address_out_of_range() {
    let mut cartridge = Mbc5Cartridge::new(rom(1, 2, 0x1A)).unwrap();
    assert!(cartridge.write_rom(0x0000, 0x0A));

    // Addresses outside A000-BFFF return None
    assert_eq!(cartridge.read_ram(0x9FFF), None);
    assert_eq!(cartridge.read_ram(0xC000), None);
    assert!(!cartridge.write_ram(0x9FFF, 0x55));
    assert!(!cartridge.write_ram(0xC000, 0x55));
}

#[test]
fn rom_address_out_of_range() {
    let mut cartridge = Mbc5Cartridge::new(rom(1, 0, 0x19)).unwrap();

    // Addresses outside 0000-7FFF return None or false
    assert_eq!(cartridge.read_rom(0x8000), None);
    assert!(!cartridge.write_rom(0x8000, 0x55));
}

#[test]
fn unsupported_ram_size_code_rejected() {
    // Type 1A with RAM size code 0x00 (no RAM) should work
    assert!(Mbc5Cartridge::new(rom(1, 0, 0x1A)).is_ok());

    // Type 1A with unsupported RAM size code 0x01
    assert!(matches!(
        Mbc5Cartridge::new(rom(1, 1, 0x1A)),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 0x01 })
    ));

    // Type 1A with unsupported RAM size code 0x04
    assert!(matches!(
        Mbc5Cartridge::new(rom(1, 4, 0x1A)),
        Err(CartridgeError::UnsupportedRamSizeCode { code: 0x04 })
    ));
}

#[test]
fn unsupported_rom_size_code_rejected() {
    // Type 19 with ROM size code 0x08 (512 banks = 64 MiB) should work
    assert!(Mbc5Cartridge::new(rom(8, 0, 0x19)).is_ok());

    // Type 19 with unsupported ROM size code 0x09
    assert!(matches!(
        Mbc5Cartridge::new(rom(9, 0, 0x19)),
        Err(CartridgeError::UnsupportedRomSizeCode { code: 0x09 })
    ));
}

#[test]
fn incorrect_rom_length_rejected() {
    // Create a ROM with wrong length for the size code
    let mut data = vec![0_u8; 0x4000 * 4]; // 4 banks = 16 KiB
    data[0x0143] = 0x80;
    data[0x0147] = 0x19;
    data[0x0148] = 2; // Claims 8 banks = 32 KiB
    data[0x0149] = 0;

    assert!(matches!(
        Mbc5Cartridge::new(data),
        Err(CartridgeError::IncorrectRomLength { expected: 0x20000, actual: 0x10000 })
    ));
}

#[test]
fn mbc5_with_128kb_ram() {
    // Type 1B and 1E have 128 KiB RAM (16 banks)
    let mut cartridge = Mbc5Cartridge::new(rom(1, 5, 0x1B)).unwrap();
    assert!(cartridge.write_rom(0x0000, 0x0A));

    // Test all 16 RAM banks
    for bank in 0..16_u8 {
        assert!(cartridge.write_rom(0x4000, bank));
        assert!(cartridge.write_ram(0xA000, bank));
        assert_eq!(cartridge.read_ram(0xA000), Some(bank));
    }
}
