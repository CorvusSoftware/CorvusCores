// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! ALU regression tests with self-authored instruction streams and BCD inputs.

use super::tests::fixture;
use super::{
    Cpu, CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO,
};

fn reference_alu(family: u8, a: u8, value: u8, carry: bool) -> (u8, u8) {
    match family {
        0 | 1 => {
            let input_carry = u16::from(family == 1 && carry);
            let result = u16::from(a) + u16::from(value) + input_carry;
            let flags = if result as u8 == 0 { FLAG_ZERO } else { 0 }
                | if u16::from(a & 15) + u16::from(value & 15) + input_carry > 15 {
                    FLAG_HALF_CARRY
                } else {
                    0
                }
                | if result > 255 { FLAG_CARRY } else { 0 };
            (result as u8, flags)
        }
        2 | 3 | 7 => {
            let input_carry = i16::from(family == 3 && carry);
            let subtrahend = i16::from(value) + input_carry;
            let result = i16::from(a) - subtrahend;
            let flags = FLAG_SUBTRACT
                | if result as u8 == 0 { FLAG_ZERO } else { 0 }
                | if i16::from(a & 15) < i16::from(value & 15) + input_carry {
                    FLAG_HALF_CARRY
                } else {
                    0
                }
                | if result < 0 { FLAG_CARRY } else { 0 };
            (if family == 7 { a } else { result as u8 }, flags)
        }
        4 => {
            let result = a & value;
            (result, FLAG_HALF_CARRY | if result == 0 { FLAG_ZERO } else { 0 })
        }
        5 => {
            let result = a ^ value;
            (result, if result == 0 { FLAG_ZERO } else { 0 })
        }
        6 => {
            let result = a | value;
            (result, if result == 0 { FLAG_ZERO } else { 0 })
        }
        _ => unreachable!("eight accumulator ALU families"),
    }
}

#[test]
fn adc_and_sbc_match_all_8bit_operands_with_and_without_carry() {
    for family in [1_u8, 3] {
        for a in 0_u16..=255 {
            for value in 0_u16..=255 {
                for carry in [false, true] {
                    let mut cpu = Cpu::new();
                    cpu.registers.a = a as u8;
                    cpu.registers.set_flags(
                        FLAG_ZERO | FLAG_SUBTRACT | FLAG_HALF_CARRY
                            | if carry { FLAG_CARRY } else { 0 },
                    );
                    cpu.alu_a(family, value as u8);
                    let (result, flags) = reference_alu(family, a as u8, value as u8, carry);
                    assert_eq!(
                        (cpu.registers.a, cpu.registers.flags()),
                        (result, flags),
                        "family {family} A={a:02X} input={value:02X} carry={carry}"
                    );
                }
            }
        }
    }
}

#[test]
fn all_64_register_alu_opcodes_decode_and_use_correct_cycles() {
    for family in 0_u8..8 {
        for source in 0_u8..8 {
            let opcode = 0x80 | (family << 3) | source;
            let (mut cpu, mut bus) = fixture(&[opcode]);
            cpu.registers.a = 0x8F;
            cpu.registers.b = 0x17;
            cpu.registers.c = 0x71;
            cpu.registers.d = 0x0F;
            cpu.registers.e = 0xFF;
            cpu.registers.set_hl(0xC123);
            cpu.registers.set_flags(FLAG_CARRY | FLAG_ZERO | FLAG_SUBTRACT);
            assert!(bus.write(0xC123, 0x23));

            let value = match source {
                0 => 0x17,
                1 => 0x71,
                2 => 0x0F,
                3 => 0xFF,
                4 => 0xC1,
                5 | 6 => 0x23,
                7 => 0x8F,
                _ => unreachable!(),
            };
            let (a, flags) = reference_alu(family, 0x8F, value, true);
            assert_eq!(cpu.step(&mut bus), Ok(if source == 6 { 8 } else { 4 }));
            assert_eq!(cpu.registers.a, a, "opcode {opcode:02X}");
            assert_eq!(cpu.registers.flags(), flags, "opcode {opcode:02X}");
            assert_eq!(cpu.registers.pc, 1);
        }
    }
}

#[test]
fn all_eight_immediate_alu_opcodes_match_register_semantics() {
    for family in 0_u8..8 {
        for carry in [false, true] {
            let opcode = 0xC6 | (family << 3);
            let (mut cpu, mut bus) = fixture(&[opcode, 0x91]);
            cpu.registers.a = 0x8F;
            cpu.registers.set_flags(if carry { FLAG_CARRY } else { 0 });
            let (a, flags) = reference_alu(family, 0x8F, 0x91, carry);
            assert_eq!(cpu.step(&mut bus), Ok(8));
            assert_eq!(cpu.registers.a, a, "opcode {opcode:02X}");
            assert_eq!(cpu.registers.flags(), flags, "opcode {opcode:02X}");
            assert_eq!(cpu.registers.pc, 2);
        }
    }
}

fn bcd(value: u16) -> u8 {
    (((value / 10) << 4) | (value % 10)) as u8
}

#[test]
fn daa_round_trips_all_pairs_of_valid_two_digit_bcd_operands() {
    for left in 0..100 {
        for right in 0..100 {
            let mut cpu = Cpu::new();
            cpu.registers.a = bcd(left);
            cpu.alu_a(0, bcd(right)); // ADD A,n8
            cpu.daa();
            let sum = left + right;
            let flags = if sum % 100 == 0 { FLAG_ZERO } else { 0 }
                | if sum >= 100 { FLAG_CARRY } else { 0 };
            assert_eq!(
                (cpu.registers.a, cpu.registers.flags()),
                (bcd(sum % 100), flags),
                "BCD addition {left} + {right}"
            );

            cpu.registers.a = bcd(left);
            cpu.alu_a(2, bcd(right)); // SUB A,n8
            cpu.daa();
            let difference = (left + 100 - right) % 100;
            let flags = FLAG_SUBTRACT
                | if difference == 0 { FLAG_ZERO } else { 0 }
                | if left < right { FLAG_CARRY } else { 0 };
            assert_eq!(
                (cpu.registers.a, cpu.registers.flags()),
                (bcd(difference), flags),
                "BCD subtraction {left} - {right}"
            );
        }
    }
}

#[test]
fn daa_opcode_and_flag_control_instructions_preserve_required_bits() {
    // DAA; CPL; SCF; CCF; CCF
    let (mut cpu, mut bus) = fixture(&[0x27, 0x2F, 0x37, 0x3F, 0x3F]);
    cpu.registers.a = 0x10;
    cpu.registers.set_flags(FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0xAA);
    assert_eq!(cpu.registers.flags(), FLAG_SUBTRACT | FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.a, 0x55);
    assert_eq!(
        cpu.registers.flags(),
        FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY
    );
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.flags(), 0);
    assert_eq!(cpu.step(&mut bus), Ok(4));
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
}

#[test]
fn accumulator_rotations_clear_zero_and_use_old_carry_correctly() {
    let cases = [
        (0x07, 0x80, 0, 0x01, FLAG_CARRY),
        (0x0F, 0x01, 0, 0x80, FLAG_CARRY),
        (0x17, 0x80, FLAG_CARRY, 0x01, FLAG_CARRY),
        (0x1F, 0x01, FLAG_CARRY, 0x80, FLAG_CARRY),
        (0x07, 0x00, FLAG_ZERO | FLAG_CARRY, 0x00, 0),
        (0x17, 0x00, FLAG_ZERO | FLAG_CARRY, 0x01, 0),
    ];
    for (opcode, initial, flags, expected, expected_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[opcode]);
        cpu.registers.a = initial;
        cpu.registers.set_flags(flags);
        assert_eq!(cpu.step(&mut bus), Ok(4));
        assert_eq!(cpu.registers.a, expected, "opcode {opcode:02X}");
        assert_eq!(cpu.registers.flags(), expected_flags, "opcode {opcode:02X}");
    }
}

#[test]
fn add_hl_16bit_sets_half_carry_from_bit_11_and_preserves_zero() {
    let cases = [
        (0x09, 0x0FFF, 0x0001, 0x1000, FLAG_ZERO | FLAG_HALF_CARRY),
        (0x19, 0xFFFF, 0x0001, 0x0000, FLAG_ZERO | FLAG_HALF_CARRY | FLAG_CARRY),
        (0x29, 0x8000, 0x8000, 0x0000, FLAG_ZERO | FLAG_CARRY),
        (0x39, 0x0FFF, 0xF000, 0xFFFF, FLAG_ZERO),
    ];
    for (opcode, hl, operand, expected, expected_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[opcode]);
        cpu.registers.set_hl(hl);
        cpu.registers.set_bc(operand);
        cpu.registers.set_de(operand);
        cpu.registers.sp = operand;
        cpu.registers.set_flags(FLAG_ZERO | FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY);
        assert_eq!(cpu.step(&mut bus), Ok(8));
        assert_eq!(cpu.registers.hl(), expected, "opcode {opcode:02X}");
        assert_eq!(cpu.registers.flags(), expected_flags, "opcode {opcode:02X}");
    }
}

#[test]
fn signed_stack_pointer_addition_and_hl_transfer_use_byte_carries() {
    let cases: [(u16, i8, u16, u8); 7] = [
        (0xFFF8, 8, 0x0000, FLAG_HALF_CARRY | FLAG_CARRY),
        (0x0000, -1, 0xFFFF, 0),
        (0x0100, -1, 0x00FF, 0),
        (0x00FF, 1, 0x0100, FLAG_HALF_CARRY | FLAG_CARRY),
        (0x000F, 1, 0x0010, FLAG_HALF_CARRY),
        (0x00F0, 16, 0x0100, FLAG_CARRY),
        (0x8000, -128, 0x7F80, 0),
    ];

    for (sp, displacement, expected, expected_flags) in cases {
        let (mut cpu, mut bus) = fixture(&[0xE8, displacement as u8]);
        cpu.registers.sp = sp;
        cpu.registers.set_flags(FLAG_ZERO | FLAG_SUBTRACT);
        assert_eq!(cpu.step(&mut bus), Ok(16));
        assert_eq!(cpu.registers.sp, expected);
        assert_eq!(cpu.registers.flags(), expected_flags);

        let (mut cpu, mut bus) = fixture(&[0xF8, displacement as u8]);
        cpu.registers.sp = sp;
        cpu.registers.set_flags(FLAG_ZERO | FLAG_SUBTRACT);
        assert_eq!(cpu.step(&mut bus), Ok(12));
        assert_eq!(cpu.registers.hl(), expected);
        assert_eq!(cpu.registers.sp, sp);
        assert_eq!(cpu.registers.flags(), expected_flags);
    }
}

#[test]
fn alu_memory_operand_reports_unimplemented_registers_without_mutation() {
    let (mut cpu, mut bus) = fixture(&[0xA6]); // AND (HL)
    cpu.registers.a = 0xFA;
    cpu.registers.set_hl(0xFF00);
    cpu.registers.set_flags(FLAG_CARRY);
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedRead { address: 0xFF00 })
    );
    assert_eq!(cpu.registers.a, 0xFA);
    assert_eq!(cpu.registers.flags(), FLAG_CARRY);
}
