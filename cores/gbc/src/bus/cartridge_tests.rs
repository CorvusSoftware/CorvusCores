// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cartridge banking through the CPU bus, OAM DMA and CGB VRAM DMA paths.

use super::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::machine::Machine;

fn mbc1_rom(banks_code: u8, ram_code: u8, cartridge_type: u8) -> Vec<u8> {
    let mut rom = vec![0; (2_usize << banks_code) * 0x4000];
    for (bank, content) in rom.chunks_mut(0x4000).enumerate() {
        content.fill(0x40 + bank as u8);
    }
    rom[0x0143] = 0x80; // native CGB mode
    rom[0x0147] = cartridge_type;
    rom[0x0148] = banks_code;
    rom[0x0149] = ram_code;
    rom
}

fn bus_with_mbc1() -> Bus {
    Bus::new(Cartridge::load(mbc1_rom(2, 3, 3)).unwrap())
}

#[test]
fn cpu_bus_routes_rom_bank_switches_and_ram_enable_to_mapper() {
    let mut bus = bus_with_mbc1();
    assert_eq!(bus.read(0x4000), Some(0x41));
    assert!(bus.write(0x2000, 3));
    assert_eq!(bus.read(0x4000), Some(0x43));
    assert_eq!(bus.read(0x0000), Some(0x40));
    assert!(bus.write(0xA000, 0xAA)); // external RAM is disabled
    assert_eq!(bus.read(0xA000), Some(0xFF));
    assert!(bus.write(0x0000, 0x0A));
    assert_eq!(bus.read(0xA000), Some(0));
    assert!(bus.write(0xA000, 0xA9));
    assert_eq!(bus.read(0xA000), Some(0xA9));

    assert!(bus.write(0x6000, 1));
    assert!(bus.write(0x4000, 2)); // advanced mode RAM bank 2
    assert_eq!(bus.read(0xA000), Some(0));
    assert!(bus.write(0xA000, 0x6D));
    assert!(bus.write(0x4000, 0)); // bank 0 still has earlier data
    assert_eq!(bus.read(0xA000), Some(0xA9));
    assert!(bus.write(0x4000, 2));
    assert_eq!(bus.read(0xA000), Some(0x6D));
}

#[test]
fn sm83_store_to_rom_address_changes_next_ld_from_banked_rom() {
    let mut rom = mbc1_rom(2, 0, 1);
    // LD A,02 ; LD (2000),A ; LD A,(4000) ; LD (C000),A ; HALT
    rom[..12].copy_from_slice(&[
        0x3E, 0x02, 0xEA, 0x00, 0x20, 0xFA, 0x00, 0x40, 0xEA, 0x00, 0xC0, 0x76,
    ]);
    let mut cpu = Cpu::new();
    let mut bus = Bus::new(Cartridge::load(rom).unwrap());
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0x4000), Some(0x42));
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xC000), Some(0x42));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.is_halted());
}

#[test]
fn machine_constructor_accepts_loaded_mapper_and_preserves_nominal_cycles() {
    let mut rom = mbc1_rom(1, 0, 1);
    // LD A,02 ; LD (2000),A ; LD A,(4000)
    rom[..8].copy_from_slice(&[0x3E, 0x02, 0xEA, 0, 0x20, 0xFA, 0, 0x40]);
    let mut machine = Machine::new(Cartridge::load(rom).unwrap());
    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.step(), Ok(16));
    assert_eq!(machine.step(), Ok(16));
    assert_eq!(machine.cpu().registers.a, 0x42);
    assert_eq!(machine.elapsed_t_cycles(), 40);
    assert_eq!(machine.cpu_t_cycles(), 40);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
}

#[test]
fn oam_dma_sources_selected_rom_bank_and_enabled_external_ram() {
    let mut bus = bus_with_mbc1();
    assert!(bus.write(0x2000, 2));
    assert!(bus.write(0xFF46, 0x40)); // copy ROM bank 2
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0x42));
    assert_eq!(bus.read(0xFE9F), Some(0x42));

    assert!(bus.write(0x0000, 0x0A));
    for i in 0_u16..160 {
        assert!(bus.write(0xA000 + i, i as u8 ^ 0xA5));
    }
    assert!(bus.write(0xFF46, 0xA0));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0xA5));
    assert_eq!(bus.read(0xFE9F), Some((159_u8) ^ 0xA5));

    assert!(bus.write(0x0000, 0x00)); // disabling RAM affects DMA source
    assert!(bus.write(0xFF46, 0xA0));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0xFF));
}

#[test]
fn gdma_reads_banked_rom_and_mapper_ram_without_cpu_memory_shortcuts() {
    let mut bus = bus_with_mbc1();
    assert!(bus.write(0x2000, 3));
    for (register, value) in [
        (0xFF51, 0x40),
        (0xFF52, 0),
        (0xFF53, 0),
        (0xFF54, 0),
        (0xFF55, 0),
    ] {
        assert!(bus.write(register, value));
    }
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0x8000), Some(0x43));
    assert_eq!(bus.read(0x800F), Some(0x43));

    assert!(bus.write(0x0000, 0x0A));
    for i in 0_u16..16 {
        assert!(bus.write(0xA000 + i, 0x80 + i as u8));
    }
    for (register, value) in [
        (0xFF51, 0xA0),
        (0xFF52, 0),
        (0xFF53, 0),
        (0xFF54, 0x10),
        (0xFF55, 0),
    ] {
        assert!(bus.write(register, value));
    }
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0x8010), Some(0x80));
    assert_eq!(bus.read(0x801F), Some(0x8F));
}
