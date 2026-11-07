// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! CPU unit tests use self-authored machine-code byte streams, not game ROMs.

use super::{Cpu, CpuError, Registers, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};
use crate::bus::Bus;
use crate::cartridge::RomOnlyCartridge;

pub(super) fn fixture(code: &[u8]) -> (Cpu, Bus) {
    assert!(code.len() < 0x0143);
    let mut rom = vec![0_u8; 0x8000];
    rom[..code.len()].copy_from_slice(code);
    rom[0x0143] = 0x80;
    let cartridge = RomOnlyCartridge::new(rom).unwrap();
    (Cpu::new(), Bus::new(cartridge))
}

#[test]
fn initial_state_is_explicitly_zero_not_a_boot_snapshot() {
    let (mut cpu, mut bus) = fixture(&[0x00]);
    assert_eq!(cpu.registers, Registers::default());
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.pc, 1);
}

#[test]
fn immediate_loads_and_register_moves_keep_flags_unchanged() {
    // LD B,42 ; LD C,05 ; LD A,B ; LD A,C
    let (mut cpu, mut bus) = fixture(&[0x06, 0x42, 0x0E, 0x05, 0x78, 0x79]);
    cpu.registers.set_flags(FLAG_ZERO | FLAG_CARRY);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x05);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
    assert_eq!(cpu.registers.pc, 6);
}

#[test]
fn loads_via_hl_and_counts_memory_cycles() {
    // LD HL,C000 ; LD (HL),5A ; LD A,(HL) ; LD (HL),A
    let (mut cpu, mut bus) = fixture(&[0x21, 0x00, 0xC0, 0x36, 0x5A, 0x7E, 0x77]);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.hl(), 0xC000);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(bus.read(0xC000), Some(0x5A));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x5A);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xC000), Some(0x5A));
}

#[test]
fn addition_and_subtraction_set_half_carry_and_carry() {
    // LD A,FF ; ADD A,01 ; SUB A,01
    let (mut cpu, mut bus) = fixture(&[0x3E, 0xFF, 0xC6, 0x01, 0xD6, 0x01]);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0);
    assert_eq!(
        cpu.registers.flags(),
        FLAG_ZERO | FLAG_HALF_CARRY | FLAG_CARRY
    );
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0xFF);
    assert_eq!(
        cpu.registers.flags(),
        FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY
    );
}

#[test]
fn inc_and_dec_preserve_the_carry_flag() {
    let (mut cpu, mut bus) = fixture(&[0x3E, 0x0F, 0x3C, 0x3D]);
    cpu.registers.set_flags(FLAG_CARRY);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x10);
    assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x0F);
    assert_eq!(
        cpu.registers.flags(),
        FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY
    );
}

#[test]
fn compare_leaves_accumulator_untouched_and_xor_resets_other_flags() {
    // LD A,0A ; LD B,0A ; CP B ; XOR 0A
    let (mut cpu, mut bus) = fixture(&[0x3E, 0x0A, 0x06, 0x0A, 0xB8, 0xEE, 0x0A]);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x0A);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_SUBTRACT);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO);
}

#[test]
fn relative_jump_supports_negative_displacements() {
    let (mut cpu, mut bus) = fixture(&[0x18, 0xFE]);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.pc, 0);
}

#[test]
fn conditional_relative_jump_has_taken_and_untaken_timings() {
    let (mut cpu, mut bus) = fixture(&[0x20, 0x02, 0x00, 0x00, 0x00]);
    cpu.registers.set_flags(FLAG_ZERO);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.pc, 2);
    cpu.registers.pc = 0;
    cpu.registers.set_flags(0);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.pc, 4);
}

#[test]
fn unconditional_and_conditional_absolute_jumps_work() {
    let (mut cpu, mut bus) = fixture(&[0xC3, 0x05, 0x00, 0x00, 0x00, 0xC2, 0x00, 0x00]);
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.pc, 5);
    cpu.registers.set_flags(FLAG_ZERO);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.pc, 8);
    cpu.registers.pc = 5;
    cpu.registers.set_flags(0);
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.pc, 0);
}

#[test]
fn sixteen_bit_increment_wraps_and_decrement_wraps_back() {
    let (mut cpu, mut bus) = fixture(&[0x21, 0xFF, 0xFF, 0x23, 0x2B]);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.hl(), 0);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.hl(), 0xFFFF);
}

#[test]
fn absolute_memory_load_and_store_use_little_endian_addresses() {
    // LD A,CC ; LD (C000),A ; LD A,00 ; LD A,(C000)
    let code = [0x3E, 0xCC, 0xEA, 0x00, 0xC0, 0x3E, 0x00, 0xFA, 0x00, 0xC0];
    let (mut cpu, mut bus) = fixture(&code);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xC000), Some(0xCC));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.a, 0xCC);
}

#[test]
fn returns_explicit_error_instead_of_faking_unimplemented_hardware() {
    let (mut cpu, mut bus) = fixture(&[0x21, 0x00, 0xFF, 0x7E]);
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedRead { address: 0xFF00 })
    );
}

#[test]
fn unsupported_halt_opcode_keeps_pc_at_faulting_instruction() {
    let (mut cpu, mut bus) = fixture(&[0x76]);
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnsupportedOpcode {
            address: 0,
            opcode: 0x76,
        })
    );
    assert_eq!(cpu.registers.pc, 0);
}
