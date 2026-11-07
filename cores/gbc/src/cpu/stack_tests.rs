// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Self-authored ROM fixtures for SM83 stack and control-flow instructions.

use super::tests::fixture;
use super::{CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

#[test]
fn push_and_pop_all_register_pairs_preserve_values_and_timing() {
    let cases = [
        (0xC5, 0xC1, 0x1234),
        (0xD5, 0xD1, 0x5678),
        (0xE5, 0xE1, 0x9ABC),
        (0xF5, 0xF1, 0xDEF0),
    ];

    for (index, (push, pop, value)) in cases.into_iter().enumerate() {
        let (mut cpu, mut bus) = fixture(&[push, pop]);
        cpu.registers.sp = 0xD000;
        match index {
            0 => cpu.registers.set_bc(value),
            1 => cpu.registers.set_de(value),
            2 => cpu.registers.set_hl(value),
            3 => cpu.registers.set_af(value),
            _ => unreachable!(),
        };
        let original_flags = cpu.registers.flags();

        assert_eq!(cpu.step(&mut bus), Ok(16));
        assert_eq!(cpu.registers.sp, 0xCFFE);
        let [high, low] = value.to_be_bytes();
        assert_eq!(bus.read(0xCFFE), Some(low));
        assert_eq!(bus.read(0xCFFF), Some(high));
        assert_eq!(cpu.registers.flags(), original_flags);

        match index {
            0 => cpu.registers.set_bc(0),
            1 => cpu.registers.set_de(0),
            2 => cpu.registers.set_hl(0),
            3 => cpu.registers.set_af(0),
            _ => unreachable!(),
        }
        assert_eq!(cpu.step(&mut bus), Ok(12));
        assert_eq!(cpu.registers.sp, 0xD000);
        let actual = match index {
            0 => cpu.registers.bc(),
            1 => cpu.registers.de(),
            2 => cpu.registers.hl(),
            3 => cpu.registers.af(),
            _ => unreachable!(),
        };
        assert_eq!(actual, value);
        assert_eq!(cpu.registers.pc, 2);
    }
}

#[test]
fn pop_af_masks_low_flag_nibble_from_stack() {
    let (mut cpu, mut bus) = fixture(&[0xF1]);
    cpu.registers.sp = 0xC000;
    assert!(bus.write(0xC000, 0xFF));
    assert!(bus.write(0xC001, 0xAB));

    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.af(), 0xABF0);
    assert_eq!(cpu.registers.sp, 0xC002);
}

#[test]
fn call_and_ret_round_trip_through_little_endian_stack() {
    // LD SP,D000 ; CALL 0008 ; NOP ; NOP ; LD B,2A ; RET
    let (mut cpu, mut bus) = fixture(&[
        0x31, 0x00, 0xD0, 0xCD, 0x08, 0x00, 0x00, 0x00, 0x06, 0x2A, 0xC9,
    ]);

    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.sp, 0xD000);
    assert_eq!(cpu.step(&mut bus), Ok(24));
    assert_eq!(cpu.registers.pc, 8);
    assert_eq!(cpu.registers.sp, 0xCFFE);
    assert_eq!(bus.read(0xCFFE), Some(6));
    assert_eq!(bus.read(0xCFFF), Some(0));

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.b, 0x2A);
    assert_eq!(cpu.step(&mut bus), Ok(16));
    assert_eq!(cpu.registers.pc, 6);
    assert_eq!(cpu.registers.sp, 0xD000);
    assert_eq!(cpu.step(&mut bus), Ok(4));
}

#[test]
fn conditional_call_all_four_conditions_taken_and_not_taken() {
    let cases = [
        (0xC4, FLAG_ZERO, 0),  // CALL NZ
        (0xCC, 0, FLAG_ZERO),  // CALL Z
        (0xD4, FLAG_CARRY, 0), // CALL NC
        (0xDC, 0, FLAG_CARRY), // CALL C
    ];
    for (opcode, false_flags, true_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[opcode, 0x08, 0x00]);
        cpu.registers.sp = 0xD000;
        cpu.registers.set_flags(false_flags);
        assert_eq!(cpu.step(&mut bus), Ok(12));
        assert_eq!(cpu.registers.pc, 3);
        assert_eq!(cpu.registers.sp, 0xD000);

        cpu.registers.pc = 0;
        cpu.registers.set_flags(true_flags);
        assert_eq!(cpu.step(&mut bus), Ok(24));
        assert_eq!(cpu.registers.pc, 8);
        assert_eq!(cpu.registers.sp, 0xCFFE);
        assert_eq!(bus.read(0xCFFE), Some(3));
        assert_eq!(bus.read(0xCFFF), Some(0));
        assert_eq!(cpu.registers.flags(), true_flags);
    }
}

#[test]
fn conditional_ret_all_four_conditions_taken_and_not_taken() {
    let cases = [
        (0xC0, FLAG_ZERO, 0),  // RET NZ
        (0xC8, 0, FLAG_ZERO),  // RET Z
        (0xD0, FLAG_CARRY, 0), // RET NC
        (0xD8, 0, FLAG_CARRY), // RET C
    ];
    for (opcode, false_flags, true_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[opcode]);
        cpu.registers.sp = 0xC000;
        assert!(bus.write(0xC000, 0x34));
        assert!(bus.write(0xC001, 0x12));

        cpu.registers.set_flags(false_flags);
        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(cpu.registers.pc, 1);
        assert_eq!(cpu.registers.sp, 0xC000);

        cpu.registers.pc = 0;
        cpu.registers.set_flags(true_flags);
        assert_eq!(cpu.step(&mut bus), Ok(20));
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.sp, 0xC002);
        assert_eq!(cpu.registers.flags(), true_flags);
    }
}

#[test]
fn all_rst_vectors_push_the_return_address() {
    for opcode in [0xC7, 0xCF, 0xD7, 0xDF, 0xE7, 0xEF, 0xF7, 0xFF] {
        let (mut cpu, mut bus) = fixture(&[opcode]);
        cpu.registers.sp = 0xC100;
        cpu.registers.set_flags(FLAG_ZERO | FLAG_CARRY);

        assert_eq!(cpu.step(&mut bus), Ok(16));
        assert_eq!(cpu.registers.pc, u16::from(opcode & 0x38));
        assert_eq!(cpu.registers.sp, 0xC0FE);
        assert_eq!(bus.read(0xC0FE), Some(1));
        assert_eq!(bus.read(0xC0FF), Some(0));
        assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
    }
}

#[test]
fn jp_hl_and_ld_sp_hl_are_register_only_and_preserve_flags() {
    let (mut cpu, mut bus) = fixture(&[0xF9, 0xE9]);
    cpu.registers.set_hl(0xC234);
    let flags = FLAG_SUBTRACT | FLAG_HALF_CARRY;
    cpu.registers.set_flags(flags);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.sp, 0xC234);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.pc, 0xC234);
    assert_eq!(cpu.registers.flags(), flags);
}

#[test]
fn ld_a16_sp_writes_low_byte_first_across_memory_regions() {
    // LD (CFFF),SP
    let (mut cpu, mut bus) = fixture(&[0x08, 0xFF, 0xCF]);
    cpu.registers.sp = 0xABCD;
    cpu.registers.set_flags(FLAG_ZERO);

    assert_eq!(cpu.step(&mut bus), Ok(20));
    assert_eq!(bus.read(0xCFFF), Some(0xCD));
    assert_eq!(bus.read(0xD000), Some(0xAB));
    assert_eq!(cpu.registers.sp, 0xABCD);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO);
}

#[test]
fn stack_errors_remain_explicit_for_unimplemented_bus_regions() {
    let (mut cpu, mut bus) = fixture(&[0xC5]); // PUSH BC
    cpu.registers.sp = 0;
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedWrite { address: 0xFFFF })
    );
    assert_eq!(cpu.registers.sp, 0xFFFF);

    let (mut cpu, mut bus) = fixture(&[0xC9]); // RET
    cpu.registers.sp = 0xFFFF;
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedRead { address: 0xFFFF })
    );
    assert_eq!(cpu.registers.sp, 0xFFFF);
}
