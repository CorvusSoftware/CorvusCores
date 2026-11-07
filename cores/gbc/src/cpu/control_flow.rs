// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 JR/JP, CALL/RET and RST control-flow instructions.
//!
//! Stack accesses remain in `stack.rs`; conditions use the original flags
//! and the CPU dispatcher remains responsible for opcode errors and EI delay.

use super::{Cpu, CpuError, FLAG_CARRY, FLAG_ZERO};
use crate::bus::Bus;

impl Cpu {
    fn check_condition(&self, opcode: u8) -> bool {
        let flags = self.registers.flags();
        // NZ, Z, NC, C share the same two-bit condition field in
        // JR cc, JP cc, CALL cc and RET cc opcodes.
        match (opcode >> 3) & 0x03 {
            0 => flags & FLAG_ZERO == 0,
            1 => flags & FLAG_ZERO != 0,
            2 => flags & FLAG_CARRY == 0,
            3 => flags & FLAG_CARRY != 0,
            _ => unreachable!("condition field is two bits"),
        }
    }

    pub(super) fn execute_control_flow(
        &mut self,
        opcode: u8,
        bus: &mut Bus,
    ) -> Result<u8, CpuError> {
        let cycles = match opcode {
            0x18 => {
                let displacement = self.fetch8(bus)? as i8;
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add(i16::from(displacement) as u16);
                12
            }
            0x20 | 0x28 | 0x30 | 0x38 => {
                let displacement = self.fetch8(bus)? as i8;
                if self.check_condition(opcode) {
                    self.registers.pc = self
                        .registers
                        .pc
                        .wrapping_add(i16::from(displacement) as u16);
                    12
                } else {
                    8
                }
            }
            0xC9 => {
                self.registers.pc = self.pop16(bus)?;
                16
            }
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                if self.check_condition(opcode) {
                    self.registers.pc = self.pop16(bus)?;
                    20
                } else {
                    8
                }
            }
            0xCD => {
                let address = self.fetch16(bus)?;
                self.push16(bus, self.registers.pc)?;
                self.registers.pc = address;
                24
            }
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let address = self.fetch16(bus)?;
                if self.check_condition(opcode) {
                    self.push16(bus, self.registers.pc)?;
                    self.registers.pc = address;
                    24
                } else {
                    12
                }
            }
            0xE9 => {
                self.registers.pc = self.registers.hl();
                4
            }
            op if op & 0xC7 == 0xC7 => {
                let vector = u16::from(opcode & 0x38);
                self.push16(bus, self.registers.pc)?;
                self.registers.pc = vector;
                16
            }
            0xC3 => {
                let address = self.fetch16(bus)?;
                self.registers.pc = address;
                16
            }
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let address = self.fetch16(bus)?;
                if self.check_condition(opcode) {
                    self.registers.pc = address;
                    16
                } else {
                    12
                }
            }
            _ => unreachable!("control-flow dispatch filters supported opcodes"),
        };
        Ok(cycles)
    }
}
