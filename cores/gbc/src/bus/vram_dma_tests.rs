// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! GDMA/HDMA integration with CGB bus, LCD modes and VRAM bank selection.

use super::Bus;
use crate::cartridge::RomOnlyCartridge;

fn fixture(cgb: bool) -> Bus {
    let mut rom = vec![0_u8; 0x8000];
    rom[0x0143] = if cgb { 0x80 } else { 0 };
    for i in 0..32 {
        rom[0x300 + i] = (0xA0 + i) as u8;
    }
    Bus::new(RomOnlyCartridge::new(rom).unwrap())
}

fn configure(bus: &mut Bus, source: u16, destination: u16) {
    for (register, value) in [
        (0xFF51, (source >> 8) as u8),
        (0xFF52, source as u8),
        (0xFF53, (destination >> 8) as u8),
        (0xFF54, destination as u8),
    ] {
        assert!(bus.write(register, value));
    }
}

#[test]
fn gdma_transfers_rom_bytes_to_selected_vram_bank_and_counts_cycles() {
    let mut bus = fixture(true);
    assert_eq!(bus.read(0xFF55), Some(0xFF));
    assert!(bus.write(0xFF4F, 1)); // Select CGB VRAM bank 1.
    configure(&mut bus, 0x030F, 0x0FF1); // masks to 0300 -> 8FF0
    assert!(bus.write(0xFF55, 0));
    assert_eq!(bus.read(0xFF55), Some(0));
    assert!(bus.vram_dma_cpu_stalled());

    bus.tick_t_cycles(31);
    assert_eq!(bus.vram.read_bank(0x8FFE, 1), Some(0xAE));
    assert_eq!(bus.vram.read_bank(0x8FFF, 1), Some(0));
    assert!(bus.vram_dma_active());
    bus.tick_t_cycles(1);
    assert_eq!(bus.vram.read_bank(0x8FFF, 1), Some(0xAF));
    assert_eq!(bus.vram.read_bank(0x8FF0, 0), Some(0));
    assert!(!bus.vram_dma_active());
    assert_eq!(bus.read(0xFF55), Some(0xFF));
}

#[test]
fn gdma_writes_vram_even_while_cpu_reads_are_blocked_in_mode_three() {
    let mut bus = fixture(true);
    for i in 0_u16..16 {
        assert!(bus.write(0xC000 + i, 0x31 + i as u8));
    }
    assert!(bus.write(0xFF40, 0x80));
    bus.tick_t_cycles(80);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 3);
    configure(&mut bus, 0xC000, 0x8000);
    assert!(bus.write(0xFF55, 0));
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0x8000), Some(0xFF));
    assert_eq!(bus.vram.read_bank(0x8000, 0), Some(0x31));
    assert_eq!(bus.vram.read_bank(0x800F, 0), Some(0x40));
    assert!(!bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(140); // mode 3 ends at LCD dot 252
    assert_eq!(bus.read(0x8000), Some(0x31));
}

#[test]
fn hdma_runs_one_block_per_visible_hblank_and_skips_vblank() {
    let mut bus = fixture(true);
    for index in 0_u16..48 {
        assert!(bus.write(0xC000 + index, (index as u8) ^ 0x55));
    }
    assert!(bus.write(0xFF40, 0x80));
    configure(&mut bus, 0xC000, 0x8000);
    assert!(bus.write(0xFF55, 0x82)); // Three blocks
    assert_eq!(bus.read(0xFF55), Some(2));
    assert!(!bus.vram_dma_cpu_stalled());

    bus.tick_t_cycles(251);
    assert_eq!(bus.vram.read_bank(0x8000, 0), Some(0));
    assert!(!bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(1); // HBlank entry, schedules the first block
    assert!(bus.vram_dma_cpu_stalled());
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 0);

    bus.tick_t_cycles(32);
    assert!(!bus.vram_dma_cpu_stalled());
    assert_eq!(bus.read(0xFF55), Some(1));
    assert_eq!(bus.vram.read_bank(0x8000, 0), Some(0x55));
    assert_eq!(bus.vram.read_bank(0x800F, 0), Some(0x5A));
    assert_eq!(bus.vram.read_bank(0x8010, 0), Some(0));

    bus.tick_t_cycles(456 - 32);
    assert_eq!(bus.read(0xFF55), Some(0)); // second block copied
    assert_eq!(bus.vram.read_bank(0x8010, 0), Some(0x45));
    bus.tick_t_cycles(32);
    assert!(!bus.vram_dma_active());
    assert_eq!(bus.read(0xFF55), Some(0xFF));
    assert_eq!(bus.vram.read_bank(0x802F, 0), Some(0x7A));
}

#[test]
fn hdma_waits_during_vblank_and_resumes_next_frame() {
    let mut bus = fixture(true);
    assert!(bus.write(0xFF40, 0x80));
    bus.tick_t_cycles(143 * 456);
    assert_eq!(bus.read(0xFF44), Some(143));
    configure(&mut bus, 0x0300, 0x9000);
    assert!(bus.write(0xFF55, 0x81));

    bus.tick_t_cycles(252);
    assert!(bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0xFF55), Some(0));
    assert_eq!(bus.vram.read_bank(0x9000, 0), Some(0xA0));

    bus.tick_t_cycles(204 - 32 + 10 * 456); // VBlank + next frame mode 2 begins
    assert_eq!(bus.read(0xFF44), Some(0));
    assert_eq!(bus.read(0xFF55), Some(0));
    assert!(!bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(252);
    assert!(bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0xFF55), Some(0xFF));
    assert_eq!(bus.vram.read_bank(0x9010, 0), Some(0xB0));
}

#[test]
fn hdma_can_be_cancelled_with_bit_seven_clear() {
    let mut bus = fixture(true);
    for i in 0_u16..32 {
        assert!(bus.write(0xC000 + i, (i + 1) as u8));
    }
    assert!(bus.write(0xFF40, 0x80));
    configure(&mut bus, 0xC000, 0x8000);
    assert!(bus.write(0xFF55, 0x83));
    bus.tick_t_cycles(252 + 32);
    assert_eq!(bus.read(0xFF55), Some(2));

    assert!(bus.write(0xFF55, 0)); // stop pending HBlank DMA
    assert!(!bus.vram_dma_active());
    assert_eq!(bus.read(0xFF55), Some(0x82));
    bus.tick_t_cycles(456 * 4);
    assert_eq!(bus.vram.read_bank(0x8010, 0), Some(0));
}

#[test]
fn dmg_compatibility_does_not_expose_cgb_dma_registers() {
    let mut bus = fixture(false);
    for address in 0xFF51..=0xFF55 {
        assert_eq!(bus.read(address), None);
        assert!(!bus.write(address, 0xFF));
    }
    assert!(!bus.vram_dma_active());
}

#[test]
fn lcd_disable_converts_pending_hdma_into_continuous_gdma() {
    let mut bus = fixture(true);
    configure(&mut bus, 0x0300, 0x8000);
    assert!(bus.write(0xFF40, 0x80));
    assert!(bus.write(0xFF55, 0x80));
    assert!(!bus.vram_dma_cpu_stalled());
    assert!(bus.write(0xFF40, 0));
    assert!(bus.vram_dma_cpu_stalled());
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0xFF55), Some(0xFF));
    assert_eq!(bus.read(0x8000), Some(0xA0));
}

#[test]
fn invalid_source_is_not_silently_read_from_cpu_vram() {
    let mut bus = fixture(true);
    assert!(bus.write(0x8000, 0x9A));
    configure(&mut bus, 0x8000, 0x9000);
    assert!(bus.write(0xFF55, 0));
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0x9000), Some(0xFF));
    assert_eq!(bus.read(0x8000), Some(0x9A));
}
