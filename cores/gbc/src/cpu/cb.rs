// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 CB-prefixed rotations, shifts, and bit operations.
//!
//! The CB opcode encoding is divided into four groups:
//! rotate/shift, BIT, RES, and SET. All 256 second-byte values are defined.
//! Cycles are nominal instruction timings; bus accesses are not timed yet.

use super::{Cpu, CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_ZERO};
use crate::bus::Bus;

impl Cpu {
    pub(super) fn execute_cb(&mut self, opcode: u8, bus: &mut Bus) -> Result<u8, CpuError> {
        let target = opcode & 0x07;
        let bit = (opcode >> 3) & 0x07;
        let value = self.read_r8(target, bus)?;

        match opcode >> 6 {
            0 => {
                let old_carry = self.registers.flags() & FLAG_CARRY != 0;
                let (result, carry) = match bit {
                    0 => (value.rotate_left(1), value & 0x80 != 0), // RLC
                    1 => (value.rotate_right(1), value & 0x01 != 0), // RRC
                    2 => ((value << 1) | u8::from(old_carry), value & 0x80 != 0), // RL
                    3 => ((value >> 1) | (u8::from(old_carry) << 7), value & 1 != 0), // RR
                    4 => (value << 1, value & 0x80 != 0),           // SLA
                    5 => ((value >> 1) | (value & 0x80), value & 1 != 0), // SRA
                    6 => (value.rotate_left(4), false),             // SWAP
                    7 => (value >> 1, value & 1 != 0),              // SRL
                    _ => unreachable!("three-bit operation field"),
                };
                self.write_r8(target, result, bus)?;

                let mut flags = 0;
                if result == 0 {
                    flags |= FLAG_ZERO;
                }
                if carry {
                    flags |= FLAG_CARRY;
                }
                self.registers.set_flags(flags);
            }
            1 => {
                // BIT preserves carry and does not modify the operand.
                let mut flags = (self.registers.flags() & FLAG_CARRY) | FLAG_HALF_CARRY;
                if value & (1 << bit) == 0 {
                    flags |= FLAG_ZERO;
                }
                self.registers.set_flags(flags);
            }
            2 => self.write_r8(target, value & !(1 << bit), bus)?, // RES
            3 => self.write_r8(target, value | (1 << bit), bus)?,  // SET
            _ => unreachable!("two-bit opcode group"),
        }

        Ok(if target == 6 {
            if opcode >> 6 == 1 {
                12 // BIT n,(HL) reads but does not write
            } else {
                16
            }
        } else {
            8
        })
    }
}
