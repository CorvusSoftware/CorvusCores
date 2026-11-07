// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration of OAM DMA, LCD access windows, and unmapped source handling.

use super::Bus;
use crate::cartridge::RomOnlyCartridge;
use crate::cpu::Cpu;

fn fixture(cgb_flag: u8) -> Bus {
    let mut rom = vec![0; 0x8000];
    rom[0x0143] = cgb_flag;
    rom[0x0200] = 0x42;
    rom[0x029F] = 0xA7;
    Bus::new(RomOnlyCartridge::new(rom).unwrap())
}

#[test]
fn dma_copies_one_byte_every_four_t_cycles_and_unlocks_oam_after_640() {
    let mut bus = fixture(0);
    for index in 0_u16..160 {
        assert!(bus.write(0xC000 + index, index as u8 ^ 0xA5));
    }
    assert!(bus.write(0xFE00, 0x55));
    assert!(bus.write(0xFF46, 0xC0));
    assert_eq!(bus.read(0xFF46), Some(0xC0));
    assert!(bus.oam_dma_active());
    assert_eq!(bus.read(0xFE00), Some(0xFF));

    bus.tick_t_cycles(3);
    assert_eq!(bus.oam.read(0xFE00), Some(0x55));
    bus.tick_t_cycles(1);
    assert_eq!(bus.oam.read(0xFE00), Some(0xA5));
    assert_eq!(bus.oam.read(0xFE01), Some(0));

    bus.tick_t_cycles(158 * 4);
    assert_eq!(bus.oam.read(0xFE9E), Some(158 ^ 0xA5));
    assert_eq!(bus.oam.read(0xFE9F), Some(0));
    assert!(bus.oam_dma_active());
    bus.tick_t_cycles(3);
    assert!(bus.oam_dma_active());
    assert_eq!(bus.oam.read(0xFE9F), Some(0));
    bus.tick_t_cycles(1);
    assert!(!bus.oam_dma_active());
    assert_eq!(bus.read(0xFE00), Some(0xA5));
    assert_eq!(bus.read(0xFE9F), Some(159 ^ 0xA5));
    assert_eq!(bus.read(0xFF46), Some(0xC0));
}

#[test]
fn dma_restart_switches_sources_and_replaces_all_oam_bytes() {
    let mut bus = fixture(0x80);
    for index in 0_u16..160 {
        assert!(bus.write(0xC000 + index, 0x11));
        assert!(bus.write(0xD000 + index, 0x80 | (index as u8 & 0x7F)));
    }
    assert!(bus.write(0xFF46, 0xC0));
    bus.tick_t_cycles(4 * 5);
    assert_eq!(bus.oam.read(0xFE00), Some(0x11));
    assert_eq!(bus.oam.read(0xFE04), Some(0x11));

    assert!(bus.write(0xFF46, 0xD0));
    bus.tick_t_cycles(3);
    assert_eq!(bus.oam.read(0xFE00), Some(0x11));
    bus.tick_t_cycles(1);
    assert_eq!(bus.oam.read(0xFE00), Some(0x80));
    bus.tick_t_cycles(4 * 159);
    assert!(!bus.oam_dma_active());
    for index in 0_u16..160 {
        assert_eq!(bus.read(0xFE00 + index), Some(0x80 | (index as u8 & 0x7F)));
    }
    assert_eq!(bus.read(0xFF46), Some(0xD0));
}

#[test]
fn dma_reads_rom_and_rom_only_absent_cart_ram_as_ff() {
    let mut bus = fixture(0x80);
    assert!(bus.write(0xFF46, 0x02));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0x42));
    assert_eq!(bus.read(0xFE9F), Some(0xA7));

    assert!(bus.write(0xFF46, 0xA0));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0xFF));
    assert_eq!(bus.read(0xFE9F), Some(0xFF));
}

#[test]
fn cpu_video_access_tracks_modes_and_lcd_off() {
    let mut bus = fixture(0x80);
    assert!(bus.write(0x8000, 0x23));
    assert!(bus.write(0xFE00, 0x45));
    assert!(bus.write(0xFF40, 0x80));
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 2);
    assert_eq!(bus.read(0x8000), Some(0x23)); // VRAM available in mode 2
    assert_eq!(bus.read(0xFE00), Some(0xFF));
    assert!(bus.write(0xFE00, 0x99)); // OAM write ignored
    assert_eq!(bus.oam.read(0xFE00), Some(0x45));

    bus.tick_t_cycles(80);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 3);
    assert_eq!(bus.read(0x8000), Some(0xFF));
    assert_eq!(bus.read(0xFE00), Some(0xFF));
    assert!(bus.write(0x8000, 0xBE));
    assert!(bus.write(0xFE00, 0xAD));
    assert_eq!(bus.vram.read(0x8000), Some(0x23));
    assert_eq!(bus.oam.read(0xFE00), Some(0x45));

    bus.tick_t_cycles(172);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 0);
    assert_eq!(bus.read(0x8000), Some(0x23));
    assert_eq!(bus.read(0xFE00), Some(0x45));
    assert!(bus.write(0x8000, 0x31));
    assert!(bus.write(0xFE00, 0x62));
    assert_eq!(bus.read(0x8000), Some(0x31));
    assert_eq!(bus.read(0xFE00), Some(0x62));

    bus.tick_t_cycles(204);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 2);
    assert_eq!(bus.read(0xFE00), Some(0xFF));

    bus.tick_t_cycles(143 * 456);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 1); // VBlank
    assert_eq!(bus.read(0x8000), Some(0x31));
    assert_eq!(bus.read(0xFE00), Some(0x62));

    assert!(bus.write(0xFF40, 0));
    assert_eq!(bus.read(0x8000), Some(0x31));
    assert_eq!(bus.read(0xFE00), Some(0x62));
}

#[test]
fn dma_bypasses_cpu_vram_mode_three_lock_and_preserves_source() {
    let mut bus = fixture(0x80);
    assert!(bus.write(0xFF4F, 1));
    assert!(bus.write(0x8000, 0xB5));
    assert!(bus.write(0xFF40, 0x80));
    bus.tick_t_cycles(80);
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 3);
    assert_eq!(bus.read(0x8000), Some(0xFF));
    assert!(bus.write(0xFF46, 0x80));
    bus.tick_t_cycles(4);
    assert_eq!(bus.oam.read(0xFE00), Some(0xB5));
    assert_eq!(bus.vram.read(0x8000), Some(0xB5));
    assert_eq!(bus.read(0xFE00), Some(0xFF)); // DMA still owns OAM
}

#[test]
fn dma_and_ppu_progress_alongside_div_and_tima() {
    let mut bus = fixture(0x80);
    assert!(bus.write(0xFF07, 0x05));
    assert!(bus.write(0xFF40, 0x80));
    assert!(bus.write(0xFF46, 0xC0));
    bus.tick_t_cycles(640);

    assert!(!bus.oam_dma_active());
    assert_eq!(bus.read(0xFF04), Some(2));
    assert_eq!(bus.read(0xFF05), Some(40));
    assert_eq!(bus.read(0xFF44), Some(1));
    assert_eq!(bus.read(0xFF41).unwrap() & 3, 3);
}

#[test]
fn normal_cpu_loads_observe_the_video_bus_read_mask() {
    let mut rom = vec![0; 0x8000];
    rom[0] = 0xFA; // LD A,(8000)
    rom[1] = 0;
    rom[2] = 0x80;
    rom[0x0143] = 0x80;
    let mut bus = Bus::new(RomOnlyCartridge::new(rom).unwrap());
    assert!(bus.write(0x8000, 0x53));
    assert!(bus.write(0xFF40, 0x80));
    bus.tick_t_cycles(80); // mode 3

    let mut cpu = Cpu::new();
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.a, 0xFF);
    assert_eq!(bus.vram.read(0x8000), Some(0x53));
}

#[test]
fn unsupported_dma_source_page_uses_documented_placeholder() {
    let mut bus = fixture(0x80);
    assert!(bus.write(0xFF46, 0xFF));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0xFF));
    assert_eq!(bus.read(0xFE9F), Some(0xFF));
}
