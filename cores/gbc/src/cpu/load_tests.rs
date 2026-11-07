// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Self-authored transfer instruction fixtures for the partial SM83 bus.

use super::tests::fixture;
use super::{CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

#[test]
fn bc_and_de_indirect_loads_and_stores_preserve_flags_and_registers() {
    // LD (BC),A ; LD (DE),A ; LD A,(BC) ; LD A,(DE)
    let (mut cpu, mut bus) = fixture(&[0x02, 0x12, 0x0A, 0x1A]);
    cpu.registers.set_bc(0xC123);
    cpu.registers.set_de(0xD234);
    cpu.registers.a = 0xA5;
    cpu.registers.set_flags(FLAG_ZERO | FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xC123), Some(0xA5));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xD234), Some(0xA5));

    assert!(bus.write(0xC123, 0x34));
    assert!(bus.write(0xD234, 0x56));
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x34);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x56);

    assert_eq!(cpu.registers.bc(), 0xC123);
    assert_eq!(cpu.registers.de(), 0xD234);
    assert_eq!(cpu.registers.pc, 4);
    assert_eq!(
        cpu.registers.flags(),
        FLAG_ZERO | FLAG_SUBTRACT | FLAG_HALF_CARRY | FLAG_CARRY
    );
}

#[test]
fn hl_auto_increment_decrement_and_load_direction_are_correct() {
    // LD (HL+),A ; LD A,(HL+) ; LD (HL-),A ; LD A,(HL-)
    let (mut cpu, mut bus) = fixture(&[0x22, 0x2A, 0x32, 0x3A]);
    cpu.registers.set_hl(0xCFFF);
    cpu.registers.a = 0x42;
    cpu.registers.set_flags(FLAG_CARRY | FLAG_ZERO);
    assert!(bus.write(0xD000, 0x73));

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xCFFF), Some(0x42));
    assert_eq!(cpu.registers.hl(), 0xD000);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x73);
    assert_eq!(cpu.registers.hl(), 0xD001);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xD001), Some(0x73));
    assert_eq!(cpu.registers.hl(), 0xD000);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x73);
    assert_eq!(cpu.registers.hl(), 0xCFFF);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
    assert_eq!(cpu.registers.pc, 4);
}

#[test]
fn hl_indexing_wraps_without_overflow_or_fake_hardware_access() {
    let (mut cpu, mut bus) = fixture(&[0x22, 0x32]);
    cpu.registers.set_hl(0xFFFE);
    cpu.registers.a = 0x91;
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xFFFE), Some(0x91));
    assert_eq!(cpu.registers.hl(), 0xFFFF);
    assert_eq!(
        cpu.step(&mut bus),
        Err(CpuError::UnmappedWrite { address: 0xFFFF })
    );
    assert_eq!(cpu.registers.hl(), 0xFFFF);

    // A write to ROM-only address 0000 is a valid no-op; HL still wraps.
    let (mut cpu, mut bus) = fixture(&[0x32, 0x3A]);
    cpu.registers.set_hl(0);
    cpu.registers.a = 0x77;
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.hl(), 0xFFFF);
    assert_eq!(bus.read(0), Some(0x32));
    cpu.registers.set_hl(0);
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x32);
    assert_eq!(cpu.registers.hl(), 0xFFFF);
}

#[test]
fn ldh_immediate_round_trips_through_high_ram() {
    let (mut cpu, mut bus) = fixture(&[0xE0, 0x80, 0xF0, 0x80]);
    cpu.registers.a = 0x99;
    cpu.registers.set_flags(FLAG_ZERO | FLAG_CARRY);

    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(bus.read(0xFF80), Some(0x99));
    cpu.registers.a = 0;
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.a, 0x99);
    assert_eq!(cpu.registers.pc, 4);
    assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_CARRY);
}

#[test]
fn ldh_c_register_round_trips_through_high_ram() {
    let (mut cpu, mut bus) = fixture(&[0xE2, 0xF2]);
    cpu.registers.c = 0xFE;
    cpu.registers.a = 0x5A;
    cpu.registers.set_flags(FLAG_SUBTRACT);

    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(bus.read(0xFFFE), Some(0x5A));
    cpu.registers.a = 0;
    assert_eq!(cpu.step(&mut bus), Ok(8));
    assert_eq!(cpu.registers.a, 0x5A);
    assert_eq!(cpu.registers.c, 0xFE);
    assert_eq!(cpu.registers.flags(), FLAG_SUBTRACT);
    assert_eq!(cpu.registers.pc, 2);
}

#[test]
fn ldh_can_address_currently_supported_cgb_video_bank_register() {
    // LDH (4F),A ; LDH A,(4F)
    let (mut cpu, mut bus) = fixture(&[0xE0, 0x4F, 0xF0, 0x4F]);
    assert!(bus.write(0x8000, 0x16));
    cpu.registers.a = 1;

    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(bus.read(0xFF4F), Some(0xFF));
    assert_eq!(bus.read(0x8000), Some(0));

    assert!(bus.write(0x8000, 0x47));
    cpu.registers.a = 0;
    assert_eq!(cpu.step(&mut bus), Ok(12));
    assert_eq!(cpu.registers.a, 0xFF);
    assert!(bus.write(0xFF4F, 0));
    assert_eq!(bus.read(0x8000), Some(0x16));
    assert_eq!(cpu.registers.pc, 4);
}

#[test]
fn unsupported_io_transfers_report_errors_and_keep_registers_intact() {
    let cases: &[(u8, &[u8], bool)] = &[
        (0xE0, &[0x00], true),
        (0xF0, &[0x00], false),
        (0xE2, &[], true),
        (0xF2, &[], false),
        (0xEA, &[0x00, 0xFF], true),
        (0xFA, &[0x00, 0xFF], false),
    ];
    for &(opcode, immediate, is_write) in cases {
        let mut code = vec![opcode];
        code.extend_from_slice(immediate);
        let (mut cpu, mut bus) = fixture(&code);
        cpu.registers.c = 0;
        cpu.registers.a = 0xA5;
        cpu.registers.set_flags(FLAG_HALF_CARRY | FLAG_CARRY);

        let expected = if is_write {
            CpuError::UnmappedWrite { address: 0xFF00 }
        } else {
            CpuError::UnmappedRead { address: 0xFF00 }
        };
        assert_eq!(cpu.step(&mut bus), Err(expected), "opcode {opcode:02X}");
        assert_eq!(cpu.registers.a, 0xA5);
        assert_eq!(cpu.registers.flags(), FLAG_HALF_CARRY | FLAG_CARRY);
        assert_eq!(cpu.registers.pc, code.len() as u16);
    }
}

#[test]
fn indirect_loads_do_not_modify_hl_or_a_when_the_bus_rejects_access() {
    for opcode in [0x02, 0x0A, 0x12, 0x1A, 0x22, 0x2A, 0x32, 0x3A] {
        let (mut cpu, mut bus) = fixture(&[opcode]);
        cpu.registers.set_bc(0xFFFF);
        cpu.registers.set_de(0xFFFF);
        cpu.registers.set_hl(0xFFFF);
        cpu.registers.a = 0xBD;
        cpu.registers.set_flags(FLAG_ZERO | FLAG_SUBTRACT);

        let expected = if [0x02, 0x12, 0x22, 0x32].contains(&opcode) {
            CpuError::UnmappedWrite { address: 0xFFFF }
        } else {
            CpuError::UnmappedRead { address: 0xFFFF }
        };
        assert_eq!(cpu.step(&mut bus), Err(expected), "opcode {opcode:02X}");
        assert_eq!(cpu.registers.hl(), 0xFFFF);
        assert_eq!(cpu.registers.a, 0xBD);
        assert_eq!(cpu.registers.flags(), FLAG_ZERO | FLAG_SUBTRACT);
        assert_eq!(cpu.registers.pc, 1);
    }
}

#[test]
fn all_transfer_opcodes_return_their_nominal_t_cycle_counts() {
    let cases: &[(u8, u8, &[u8])] = &[
        (0x02, 8, &[]),
        (0x0A, 8, &[]),
        (0x12, 8, &[]),
        (0x1A, 8, &[]),
        (0x22, 8, &[]),
        (0x2A, 8, &[]),
        (0x32, 8, &[]),
        (0x3A, 8, &[]),
        (0xE0, 12, &[0x80]),
        (0xF0, 12, &[0x80]),
        (0xE2, 8, &[]),
        (0xF2, 8, &[]),
        (0xEA, 16, &[0x00, 0xC0]),
        (0xFA, 16, &[0x00, 0xC0]),
    ];
    for &(opcode, cycles, operands) in cases {
        let mut code = vec![opcode];
        code.extend_from_slice(operands);
        let (mut cpu, mut bus) = fixture(&code);
        cpu.registers.set_bc(0xC080);
        cpu.registers.set_de(0xD123);
        cpu.registers.set_hl(0xC456);
        cpu.registers.a = 0x77;
        cpu.registers.set_flags(FLAG_CARRY);

        assert_eq!(cpu.step(&mut bus), Ok(cycles), "opcode {opcode:02X}");
        assert_eq!(cpu.registers.pc, code.len() as u16);
        assert_eq!(cpu.registers.flags(), FLAG_CARRY);
    }
}
