// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! MBC3 through CPU-visible bus, DMA sources and Machine RTC input.

use super::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::machine::Machine;

fn rom(kind: u8, ram_code: u8) -> Vec<u8> {
    let mut rom = vec![0; 4 * 0x4000];
    for (index, bank) in rom.chunks_mut(0x4000).enumerate() {
        bank.fill(0x30 + index as u8);
    }
    rom[0x0143] = 0x80;
    rom[0x0147] = kind;
    rom[0x0148] = 1;
    rom[0x0149] = ram_code;
    rom
}

fn rtc_bus() -> Bus {
    Bus::new(Cartridge::load(rom(0x10, 3)).unwrap())
}

fn rtc_latch(bus: &mut Bus) {
    assert!(bus.write(0x6000, 0));
    assert!(bus.write(0x6000, 1));
}

#[test]
fn sm83_instructions_switch_mbc3_rom_banks_without_bus_mapper_logic() {
    let mut data = rom(0x11, 0);
    // LD A,03 ; LD (2000),A ; LD A,(4000) ; LD (C000),A
    data[..11].copy_from_slice(&[0x3E, 3, 0xEA, 0, 0x20, 0xFA, 0, 0x40, 0xEA, 0, 0xC0]);
    let mut cpu = Cpu::new();
    let mut bus = Bus::new(Cartridge::load(data).unwrap());
    for cycles in [8, 16, 16, 16] {
        assert_eq!(cpu.step(&mut bus), Ok(cycles));
    }
    assert_eq!(bus.read(0x4000), Some(0x33));
    assert_eq!(bus.read(0xC000), Some(0x33));
    assert_eq!(cpu.registers.a, 0x33);
}

#[test]
fn bus_rtc_latch_requires_write_sequence_and_explicit_seconds() {
    let mut bus = rtc_bus();
    assert!(bus.write(0x0000, 0x0A));
    assert!(bus.write(0x4000, 0x08)); // RTC seconds
    bus.advance_rtc_seconds(58);
    assert_eq!(bus.read(0xA000), Some(0)); // initial snapshot

    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xA000), Some(58));
    bus.tick_t_cycles(2048); // CPU/PPU time does not imply RTC seconds
    assert_eq!(bus.read(0xA000), Some(58));
    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xBFFF), Some(58));

    bus.advance_rtc_seconds(4);
    assert_eq!(bus.read(0xA000), Some(58));
    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xA000), Some(2));
    assert!(bus.write(0x4000, 0x09));
    assert_eq!(bus.read(0xA000), Some(1));
}

#[test]
fn machine_accepts_mbc3_and_advances_rtc_without_mutating_cpu_counters() {
    let mut data = rom(0x0F, 0);
    data[0] = 0x76; // HALT
    let mut machine = Machine::new(Cartridge::load(data).unwrap());
    assert!(machine.bus_mut().write(0x0000, 0x0A));
    assert!(machine.bus_mut().write(0x4000, 0x08));

    machine.advance_rtc_seconds(3_661);
    assert_eq!(machine.elapsed_t_cycles(), 0);
    rtc_latch(machine.bus_mut());
    assert_eq!(machine.bus().read(0xA000), Some(1));
    assert!(machine.bus_mut().write(0x4000, 0x09));
    assert_eq!(machine.bus().read(0xA000), Some(1));
    assert!(machine.bus_mut().write(0x4000, 0x0A));
    assert_eq!(machine.bus().read(0xA000), Some(1));

    assert_eq!(machine.step(), Ok(4)); // HALT
    for _ in 0..20 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 84);
    assert_eq!(machine.cpu_t_cycles(), 84);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
    assert!(machine.bus_mut().write(0x4000, 0x08));
    rtc_latch(machine.bus_mut());
    assert_eq!(machine.bus().read(0xA000), Some(1)); // no RTC change from HALT
}

#[test]
fn oam_dma_reads_active_mbc3_ram_bank_and_latched_rtc_registers() {
    let mut bus = rtc_bus();
    assert!(bus.write(0x0000, 0x0A));
    assert!(bus.write(0x4000, 3)); // bank 3 of external SRAM
    for index in 0_u16..160 {
        assert!(bus.write(0xA000 + index, index as u8 ^ 0x5A));
    }

    assert!(bus.write(0xFF46, 0xA0));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(0x5A));
    assert_eq!(bus.read(0xFE9F), Some(159_u8 ^ 0x5A));

    assert!(bus.write(0x4000, 0x08)); // RTC seconds
    bus.advance_rtc_seconds(29);
    rtc_latch(&mut bus);
    assert!(bus.write(0xFF46, 0xA0));
    bus.tick_t_cycles(640);
    assert_eq!(bus.read(0xFE00), Some(29));
    assert_eq!(bus.read(0xFE9F), Some(29));
}

#[test]
fn cgb_gdma_reads_mbc3_rtc_via_same_mapper_memory_window() {
    let mut bus = rtc_bus();
    assert!(bus.write(0x0000, 0x0A));
    assert!(bus.write(0x4000, 0x08));
    bus.advance_rtc_seconds(37);
    rtc_latch(&mut bus);
    for (address, value) in [
        (0xFF51, 0xA0),
        (0xFF52, 0),
        (0xFF53, 0),
        (0xFF54, 0),
        (0xFF55, 0),
    ] {
        assert!(bus.write(address, value));
    }
    bus.tick_t_cycles(32);
    assert_eq!(bus.read(0x8000), Some(37));
    assert_eq!(bus.read(0x800F), Some(37));
    assert!(!bus.vram_dma_active());
}

#[test]
fn bank_switching_rtc_halt_and_disable_propagate_through_bus() {
    let mut bus = rtc_bus();
    assert!(bus.write(0x0000, 0x0A));
    assert!(bus.write(0x4000, 0x0C));
    assert!(bus.write(0xA000, 0x40)); // HALT RTC
    bus.advance_rtc_seconds(10_000);
    assert!(bus.write(0x4000, 0x08));
    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xA000), Some(0));

    assert!(bus.write(0x4000, 0x0C));
    assert!(bus.write(0xA000, 0)); // resume
    bus.advance_rtc_seconds(64);
    assert!(bus.write(0x4000, 0x08));
    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xA000), Some(4));
    assert!(bus.write(0x0000, 0)); // access disabled, oscillator still runs
    bus.advance_rtc_seconds(1);
    assert_eq!(bus.read(0xA000), Some(0xFF));
    assert!(bus.write(0x0000, 0x0A));
    rtc_latch(&mut bus);
    assert_eq!(bus.read(0xA000), Some(5));
}
