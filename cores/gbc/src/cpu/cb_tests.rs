// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Table-driven regression tests for the complete SM83 CB opcode space.
//! Fixtures are synthetic ROM byte arrays; no commercial game code is used.

use super::tests::fixture;
use super::{CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

#[test]
fn all_256_cb_opcodes_are_decoded_with_correct_nominal_cycles() {
    for opcode in 0_u16..=255 {
        let (mut cpu, mut bus) = fixture(&[0xCB, opcode as u8]);
        cpu.registers.set_hl(0xC000);
        assert!(bus.write(0xC000, 0x81));

        let expected_cycles = if opcode & 0x07 == 6 {
            if (0x40..=0x7F).contains(&opcode) {
                12 // BIT n,(HL) only reads memory
            } else {
                16
            }
        } else {
            8
        };

        assert_eq!(cpu.step(&mut bus), Ok(expected_cycles), "CB {opcode:02X}");
        assert_eq!(cpu.registers.pc, 2, "CB {opcode:02X}");
    }
}

#[test]
fn rotates_shifts_and_swap_update_carry_zero_and_result() {
    // (CB opcode on B, input, incoming carry, result, expected flags)
    let examples = [
        (0x00, 0x80, false, 0x01, FLAG_CARRY),             // RLC B
        (0x08, 0x01, false, 0x80, FLAG_CARRY),             // RRC B
        (0x10, 0x80, true, 0x01, FLAG_CARRY),              // RL B
        (0x18, 0x01, true, 0x80, FLAG_CARRY),              // RR B
        (0x20, 0x81, false, 0x02, FLAG_CARRY),             // SLA B
        (0x28, 0x81, false, 0xC0, FLAG_CARRY),             // SRA B
        (0x30, 0xF0, true, 0x0F, 0),                       // SWAP B
        (0x38, 0x01, false, 0x00, FLAG_ZERO | FLAG_CARRY), // SRL B
        (0x00, 0x00, true, 0x00, FLAG_ZERO),               // RLC zero
        (0x10, 0x00, true, 0x01, 0),                       // RL through carry
        (0x18, 0x00, true, 0x80, 0),                       // RR through carry
    ];

    for (opcode, input, old_carry, result, flags) in examples {
        let (mut cpu, mut bus) = fixture(&[0xCB, opcode]);
        cpu.registers.b = input;
        cpu.registers
            .set_flags(if old_carry { FLAG_CARRY } else { 0 });
        assert_eq!(cpu.step(&mut bus), Ok(8), "CB {opcode:02X}");
        assert_eq!(cpu.registers.b, result, "CB {opcode:02X}");
        assert_eq!(cpu.registers.flags(), flags, "CB {opcode:02X}");
    }
}

#[test]
fn bit_tests_all_positions_without_writing_or_destroying_carry() {
    for bit in 0_u8..8 {
        let opcode = 0x40 | (bit << 3); // BIT bit,B
        let (mut cpu, mut bus) = fixture(&[0xCB, opcode]);
        cpu.registers.b = 1 << bit;
        cpu.registers
            .set_flags(FLAG_SUBTRACT | FLAG_CARRY | FLAG_ZERO);

        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(cpu.registers.b, 1 << bit);
        assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);

        cpu.registers.pc = 0;
        cpu.registers.b = 0;
        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(
            cpu.registers.flags(),
            FLAG_ZERO | FLAG_HALF_CARRY | FLAG_CARRY
        );
    }
}

#[test]
fn res_and_set_each_bit_preserving_all_flags() {
    for bit in 0_u8..8 {
        let flags = FLAG_ZERO | FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY;
        let (mut cpu, mut bus) = fixture(&[0xCB, 0x80 | (bit << 3)]);
        cpu.registers.b = 0xFF;
        cpu.registers.set_flags(flags);
        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(cpu.registers.b, !(1 << bit));
        assert_eq!(cpu.registers.flags(), flags);

        let (mut cpu, mut bus) = fixture(&[0xCB, 0xC0 | (bit << 3)]);
        cpu.registers.b = 0;
        cpu.registers.set_flags(flags);
        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(cpu.registers.b, 1 << bit);
        assert_eq!(cpu.registers.flags(), flags);
    }
}

#[test]
fn hl_memory_variants_use_the_bus_and_distinct_bit_timing() {
    // RLC (HL); BIT 0,(HL); RES 0,(HL); SET 0,(HL)
    let (mut cpu, mut bus) = fixture(&[0xCB, 0x06, 0xCB, 0x46, 0xCB, 0x86, 0xCB, 0xC6]);
    cpu.registers.set_hl(0xC000);
    assert!(bus.write(0xC000, 0x80));

    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xC000), Some(0x01));
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(bus.read(0xC000), Some(0x01));
    assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xC000), Some(0));
    assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(bus.read(0xC000), Some(1));
    assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);
    assert_eq!(cpu.registers.pc, 8);
}

#[test]
fn unavailable_hardware_is_an_explicit_cb_read_error() {
    let (mut cpu, mut bus) = fixture(&[0xCB, 0x46]); // BIT 0,(HL)
    cpu.registers.set_hl(0xFF00); // Joypad not modeled yet
    cpu.registers.set_flags(FLAG_CARRY);

    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedRead { address: 0xFF00 })
    );
    assert_eq!(cpu.registers.pc, 2);
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
}
