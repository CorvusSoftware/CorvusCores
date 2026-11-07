// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic instruction streams covering SM83 IME, interrupt dispatch and HALT.

use super::tests::fixture;
use super::{CpuError, FLAG_CARRY, FLAG_ZERO};
use crate::interrupts::Interrupt;

#[test]
fn ei_enables_ime_after_exactly_one_following_instruction() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x00, 0x00]); // EI; NOP; NOP
    cpu.registers.sp = 0xC100;
    cpu.registers.set_flags(FLAG_ZERO | FLAG_CARRY);
    bus.request_interrupt(Interrupt::Timer);
    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));

    assert!(!cpu.interrupts_enabled());
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(!cpu.interrupts_enabled());
    assert_eq!(cpu.registers.pc, 1);
    assert_eq!(bus.read(0xFF0F), Some(0xE4));

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.interrupts_enabled());
    assert_eq!(cpu.registers.pc, 2);

    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(cpu.registers.pc, 0x0050);
    assert_eq!(cpu.registers.sp, 0xC0FE);
    assert_eq!(bus.read(0xC0FE), Some(2));
    assert_eq!(bus.read(0xC0FF), Some(0));
    assert_eq!(bus.read(0xFF0F), Some(0xE0));
    assert_eq!(bus.read(0xFFFF), Some(Interrupt::Timer.mask()));
    assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
    assert!(!cpu.interrupts_enabled());
}

#[test]
fn di_cancels_a_pending_ei_and_keeps_interrupt_requested() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0xF3, 0x00]);
    bus.request_interrupt(Interrupt::Timer);
    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(!cpu.interrupts_enabled());
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(!cpu.interrupts_enabled());
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.pc, 3);
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
}

#[test]
fn consecutive_ei_instructions_enable_ime_after_the_second_ei() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0xFB, 0x00]);
    cpu.registers.sp = 0xC100;
    assert!(bus.write(0xFFFF, 1));
    bus.request_interrupt(Interrupt::VBlank);

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(!cpu.interrupts_enabled());
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.interrupts_enabled());
    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(cpu.registers.pc, 0x0040);
}

#[test]
fn reti_pops_return_address_and_immediately_reenables_interrupts() {
    let (mut cpu, mut bus) = fixture(&[0xD9]);
    cpu.registers.sp = 0xC000;
    cpu.registers.set_flags(FLAG_CARRY);
    assert!(bus.write(0xC000, 0x34));
    assert!(bus.write(0xC001, 0x12));

    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.pc, 0x1234);
    assert_eq!(cpu.registers.sp, 0xC002);
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
    assert!(cpu.interrupts_enabled());

    bus.request_interrupt(Interrupt::Serial);
    assert!(bus.write(0xFFFF, Interrupt::Serial.mask()));
    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(cpu.registers.pc, Interrupt::Serial.vector());
    assert!(!cpu.interrupts_enabled());
    assert_eq!(bus.read(0xC000), Some(0x34));
    assert_eq!(bus.read(0xC001), Some(0x12));
}

#[test]
fn interrupt_priority_and_ime_prevent_nested_service() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x00]);
    cpu.registers.sp = 0xD000;
    assert!(bus.write(0xFFFF, 0x1F));
    bus.request_interrupt(Interrupt::Joypad);
    bus.request_interrupt(Interrupt::Timer);
    bus.request_interrupt(Interrupt::VBlank);

    assert_eq!(cpu.step(&mut bus), Ok(4)); // EI
    assert_eq!(cpu.step(&mut bus), Ok(4)); // NOP
    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(cpu.registers.pc, Interrupt::VBlank.vector());
    assert_eq!(bus.read(0xFF0F), Some(0xF4));
    assert!(!cpu.interrupts_enabled());

    assert_eq!(cpu.step(&mut bus), Ok(4)); // handler NOP, no nested IRQ
    assert_eq!(cpu.registers.pc, Interrupt::VBlank.vector() + 1);
    assert_eq!(bus.highest_pending_interrupt(), Some(Interrupt::Timer));
}

#[test]
fn halt_with_ime_off_wakes_without_servicing_an_enabled_interrupt() {
    let (mut cpu, mut bus) = fixture(&[0x76, 0x00]);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.is_halted());
    assert_eq!(cpu.registers.pc, 1);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.pc, 1);

    bus.request_interrupt(Interrupt::Timer);
    assert_eq!(cpu.step(&mut bus), Ok(4)); // IE remains zero
    assert!(cpu.is_halted());

    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));
    assert_eq!(cpu.step(&mut bus), Ok(4)); // NOP at PC=1
    assert!(!cpu.is_halted());
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
}

#[test]
fn halt_with_ime_on_services_interrupt_before_next_opcode() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x00, 0x76, 0x00]);
    cpu.registers.sp = 0xD000;
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.interrupts_enabled());

    assert_eq!(cpu.step(&mut bus), Ok(4)); // HALT
    assert!(cpu.is_halted());
    assert_eq!(cpu.step(&mut bus), Ok(4)); // no IRQ yet
    assert_eq!(cpu.registers.pc, 3);

    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));
    bus.request_interrupt(Interrupt::Timer);
    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert!(!cpu.is_halted());
    assert_eq!(cpu.registers.pc, Interrupt::Timer.vector());
    assert_eq!(bus.read(0xCFFE), Some(3));
    assert_eq!(bus.read(0xCFFF), Some(0));
    assert!(!cpu.interrupts_enabled());
}

#[test]
fn halt_bug_doubles_opcode_fetch_when_ime_is_off_and_irq_pending() {
    let (mut cpu, mut bus) = fixture(&[0x76, 0x04, 0x00]); // HALT; INC B; NOP
    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));
    bus.request_interrupt(Interrupt::Timer);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(!cpu.is_halted());
    assert_eq!(cpu.registers.pc, 1);

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.b, 1);
    assert_eq!(cpu.registers.pc, 1);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.b, 2);
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
}

#[test]
fn halt_bug_can_make_opcode_byte_become_its_own_immediate_operand() {
    let (mut cpu, mut bus) = fixture(&[0x76, 0x06, 0x42]); // HALT; LD B,42
    assert!(bus.write(0xFFFF, 1));
    bus.request_interrupt(Interrupt::VBlank);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.b, 0x06);
    assert_eq!(cpu.registers.pc, 2);
}

#[test]
fn ei_halt_with_a_pending_irq_pushes_the_halt_address() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x76, 0x00]);
    cpu.registers.sp = 0xC100;
    assert!(bus.write(0xFFFF, 1));
    bus.request_interrupt(Interrupt::VBlank);

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.interrupts_enabled());
    assert!(!cpu.is_halted());

    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(cpu.registers.pc, 0x0040);
    assert_eq!(bus.read(0xC0FE), Some(1));
    assert_eq!(bus.read(0xC0FF), Some(0));
}

#[test]
fn a_failed_interrupt_stack_write_keeps_if_and_ime_unchanged() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x00, 0x00]);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert!(cpu.interrupts_enabled());
    cpu.registers.sp = 0xFF04; // PUSH high byte would access unmapped FF03
    assert!(bus.write(0xFFFF, Interrupt::Timer.mask()));
    bus.request_interrupt(Interrupt::Timer);

    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedWrite { address: 0xFF03 })
    );
    assert_eq!(cpu.registers.sp, 0xFF03);
    assert_eq!(cpu.registers.pc, 2);
    assert!(cpu.interrupts_enabled());
    assert_eq!(bus.read(0xFF0F), Some(0xE4));
}

#[test]
fn cpu_does_not_implicitly_tick_the_divider_during_halt_or_irq() {
    let (mut cpu, mut bus) = fixture(&[0xFB, 0x00, 0x76]);
    cpu.registers.sp = 0xC100;
    assert!(bus.write(0xFF07, 0x05));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.step(&mut bus), Ok(4));

    assert!(bus.write(0xFFFF, 1));
    bus.request_interrupt(Interrupt::VBlank);
    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(bus.read(0xFF05), Some(0));
    bus.tick_t_cycles(16);
    assert_eq!(bus.read(0xFF05), Some(1));
}
