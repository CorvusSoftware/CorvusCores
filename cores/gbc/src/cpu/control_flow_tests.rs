// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Regression coverage for the extracted SM83 control-flow dispatcher.

use super::tests::fixture;
use super::{CpuError, FLAG_CARRY, FLAG_ZERO};

#[test]
fn relative_jumps_preserve_flags_and_wrap_negative_offsets() {
    let (mut cpu, mut bus) = fixture(&[0x18, 0xFE]); // JR -2
    cpu.registers.set_flags(FLAG_ZERO | FLAG_CARRY);
    for _ in 0..3 {
        assert_eq!(cpu.step(&mut bus), Ok(12));
        assert_eq!(cpu.registers.pc, 0);
        assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
    }

    let (mut cpu, mut bus) = fixture(&[0x20, 0x80]); // JR NZ,-128
    cpu.registers.set_flags(FLAG_ZERO);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.pc, 2); // not taken, still consumes immediate

    cpu.registers.pc = 0;
    cpu.registers.set_flags(FLAG_CARRY); // NZ now true
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.pc, 0xFF82); // 0x0002 - 128 wraps as u16
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
}

#[test]
fn jp_condition_decoding_preserves_pc_flags_and_both_cycle_counts() {
    let cases = [
        (0xC2, FLAG_ZERO, 0),  // JP NZ
        (0xCA, 0, FLAG_ZERO),  // JP Z
        (0xD2, FLAG_CARRY, 0), // JP NC
        (0xDA, 0, FLAG_CARRY), // JP C
    ];
    for (opcode, not_taken_flags, taken_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[opcode, 0x34, 0x12]);
        cpu.registers.set_flags(not_taken_flags);
        assert_eq!(cpu.step(&mut bus), Ok(12), "opcode {opcode:02X} skip");
        assert_eq!(cpu.registers.pc, 3);
        assert_eq!(cpu.registers.flags(), not_taken_flags);

        cpu.registers.pc = 0;
        cpu.registers.set_flags(taken_flags);
        assert_eq!(cpu.step(&mut bus), Ok(16), "opcode {opcode:02X} take");
        assert_eq!(cpu.registers.pc, 0x1234);
        assert_eq!(cpu.registers.flags(), taken_flags);
    }
}

#[test]
fn call_stack_failure_preserves_partial_access_and_fault_address() {
    let (mut cpu, mut bus) = fixture(&[0xCD, 0x34, 0x12]); // CALL 1234
    cpu.registers.sp = 0xFF04; // first push write: FF03, unsupported I/O
    cpu.registers.set_flags(FLAG_CARRY);
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedWrite { address: 0xFF03 })
    );
    assert_eq!(cpu.registers.pc, 3); // fetched immediate before failed push
    assert_eq!(cpu.registers.sp, 0xFF03);
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
}
