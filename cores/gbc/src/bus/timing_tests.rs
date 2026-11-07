// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Bus-level integration of timer, IF/IE and CPU memory instructions.

use super::Bus;
use crate::cartridge::RomOnlyCartridge;
use crate::cpu::Cpu;
use crate::interrupts::Interrupt;

fn fixture(program: &[u8]) -> Bus {
    assert!(program.len() < 0x0143);
    let mut rom = vec![0; 0x8000];
    rom[..program.len()].copy_from_slice(program);
    rom[0x0143] = 0x80;
    Bus::new(RomOnlyCartridge::new(rom).unwrap())
}

#[test]
fn all_timer_and_interrupt_registers_are_memory_mapped() {
    let mut bus = fixture(&[]);
    assert_eq!(bus.read(0xFF04), Some(0));
    assert_eq!(bus.read(0xFF05), Some(0));
    assert_eq!(bus.read(0xFF06), Some(0));
    assert_eq!(bus.read(0xFF07), Some(0xF8));
    assert_eq!(bus.read(0xFF0F), Some(0xE0));
    assert_eq!(bus.read(0xFFFF), Some(0));

    assert!(bus.write(0xFF06, 0x41));
    assert!(bus.write(0xFF05, 0x12));
    assert!(bus.write(0xFF07, 0x05));
    assert!(bus.write(0xFF0F, 0x04));
    assert!(bus.write(0xFFFF, 0x04));

    assert_eq!(bus.read(0xFF06), Some(0x41));
    assert_eq!(bus.read(0xFF05), Some(0x12));
    assert_eq!(bus.read(0xFF07), Some(0xFD));
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
    assert_eq!(bus.read(0xFFFF), Some(0x04));
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Timer));

    assert!(bus.write(0xFF0F, 0));
    assert_eq!(bus.highest_pending_interrupt(), None);
    assert_eq!(bus.read(0xFFFF), Some(0x04));
}

#[test]
fn timer_irq_is_requested_independently_of_ie() {
    let mut bus = fixture(&[]);
    assert!(bus.write(0xFF05, 0xFF));
    assert!(bus.write(0xFF06, 0xC0));
    assert!(bus.write(0xFF07, 0x05));

    bus.tick_t_cycles(16);
    assert_eq!(bus.read(0xFF05), Some(0));
    assert_eq!(bus.read(0xFF0F), Some(0xE0));
    bus.tick_t_cycles(4);
    assert_eq!(bus.read(0xFF05), Some(0xC0));
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
    assert_eq!(bus.highest_pending_interrupt(), None);

    assert!(bus.write(0xFFFF, 0x04));
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Timer));
    bus.acknowledge_interrupt(Interrupt::Timer);
    assert_eq!(bus.read(0xFF0F), Some(0xE0));
    assert_eq!(bus.highest_pending_interrupt(), None);
}

#[test]
fn cpu_ldh_and_absolute_loads_can_access_if_and_ie() {
    // LD A,04 ; LDH (0F),A ; LD (FFFF),A ; LDH A,(0F)
    let mut bus = fixture(&[0x3E, 0x04, 0xE0, 0x0F, 0xEA, 0xFF, 0xFF, 0xF0, 0x0F]);
    let mut cpu = Cpu::new();

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xFFFF), Some(0x04));
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.a, 0xE4);
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Timer));
}

#[test]
fn manual_timer_ticking_is_not_implicitly_done_by_cpu_step() {
    let mut bus = fixture(&[0x00]);
    let mut cpu = Cpu::new();
    assert!(bus.write(0xFF07, 0x05));

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(bus.read(0xFF05), Some(0));
    bus.tick_t_cycles(4);
    assert_eq!(bus.read(0xFF05), Some(0));
    bus.tick_t_cycles(12);
    assert_eq!(bus.read(0xFF05), Some(1));
}

#[test]
fn request_priority_is_observable_without_premature_cpu_dispatch() {
    let mut bus = fixture(&[]);
    assert!(bus.write(0xFFFF, 0x1F));
    bus.request_interrupt(Interrupt::Joypad);
    bus.request_interrupt(Interrupt::Timer);
    bus.request_interrupt(Interrupt::VBlank);

    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::VBlank));
    bus.acknowledge_interrupt(Interrupt::VBlank);
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Timer));
    bus.acknowledge_interrupt(Interrupt::Timer);
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Joypad));
}

#[test]
fn unsupported_io_registers_remain_unimplemented() {
    let mut bus = fixture(&[]);
    for address in [0xFF00, 0xFF01, 0xFF02, 0xFF50] {
        assert_eq!(bus.read(address), None);
        assert!(!bus.write(address, 0xFF));
    }
}
