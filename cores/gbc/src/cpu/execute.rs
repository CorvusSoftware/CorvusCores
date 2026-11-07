// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! An intentionally small SM83 interpreter with nominal T-cycle counts.
//!
//! The bus is not cycle-accurate yet; the returned count is instruction
//! timing only, not proof that the surrounding machine has advanced.

use super::{Cpu, CpuError, FLAG_CARRY, FLAG_ZERO};
use crate::bus::Bus;

impl Cpu {
    pub(super) fn fetch8(&mut self, bus: &Bus) -> Result<u8, CpuError> {
        let address = self.registers.pc;
        let value = bus
            .read(address)
            .ok_or(CpuError::UnmappedRead { address })?;
        self.registers.pc = address.wrapping_add(1);
        Ok(value)
    }

    pub(super) fn fetch16(&mut self, bus: &Bus) -> Result<u16, CpuError> {
        let low = self.fetch8(bus)?;
        let high = self.fetch8(bus)?;
        Ok(u16::from_le_bytes([low, high]))
    }

    pub(super) fn read_r8(&self, code: u8, bus: &Bus) -> Result<u8, CpuError> {
        let r = &self.registers;
        match code {
            0 => Ok(r.b),
            1 => Ok(r.c),
            2 => Ok(r.d),
            3 => Ok(r.e),
            4 => Ok(r.h),
            5 => Ok(r.l),
            6 => bus
                .read(r.hl())
                .ok_or(CpuError::UnmappedRead { address: r.hl() }),
            7 => Ok(r.a),
            _ => unreachable!("8-bit register field is three bits"),
        }
    }

    pub(super) fn write_r8(&mut self, code: u8, value: u8, bus: &mut Bus) -> Result<(), CpuError> {
        let r = &mut self.registers;
        match code {
            0 => r.b = value,
            1 => r.c = value,
            2 => r.d = value,
            3 => r.e = value,
            4 => r.h = value,
            5 => r.l = value,
            6 => {
                let address = r.hl();
                if !bus.write(address, value) {
                    return Err(CpuError::UnmappedWrite { address });
                }
            }
            7 => r.a = value,
            _ => unreachable!("8-bit register field is three bits"),
        }
        Ok(())
    }

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

    /// Executes one supported opcode and returns its nominal number of T-cycles.
    ///
    /// This is not a transactional operation if a later operand or data access
    /// fails: the PC (and possibly registers) may already have advanced.
    /// Unsupported opcodes restore the PC to the opcode address.
    pub fn step(&mut self, bus: &mut Bus) -> Result<u8, CpuError> {
        let opcode_address = self.registers.pc;
        let opcode = self.fetch8(bus)?;

        let cycles = match opcode {
            0x00 => 4, // NOP
            0xCB => {
                let cb_opcode = self.fetch8(bus)?;
                self.execute_cb(cb_opcode, bus)?
            }
            0x02 | 0x12 | 0x0A | 0x1A | 0x22 | 0x32 | 0x2A | 0x3A | 0xE0 | 0xF0
            | 0xE2 | 0xF2 | 0xEA | 0xFA => self.execute_transfer(opcode, bus)?,
            0x01 | 0x11 | 0x21 | 0x31 => {
                let value = self.fetch16(bus)?;
                self.registers.write_pair((opcode >> 4) & 0x03, value);
                12
            }
            0x03 | 0x13 | 0x23 | 0x33 => {
                let pair = (opcode >> 4) & 0x03;
                let result = self.registers.read_pair(pair).wrapping_add(1);
                self.registers.write_pair(pair, result);
                8
            }
            0x07 | 0x0F | 0x17 | 0x1F => {
                self.rotate_accumulator(opcode);
                4
            }
            0x09 | 0x19 | 0x29 | 0x39 => {
                let value = self.registers.read_pair((opcode >> 4) & 0x03);
                self.add_hl(value);
                8
            }
            0x27 => {
                self.daa();
                4
            }
            0x2F => {
                self.complement_a();
                4
            }
            0x37 => {
                self.set_carry();
                4
            }
            0x3F => {
                self.complement_carry();
                4
            }
            0x0B | 0x1B | 0x2B | 0x3B => {
                let pair = (opcode >> 4) & 0x03;
                let result = self.registers.read_pair(pair).wrapping_sub(1);
                self.registers.write_pair(pair, result);
                8
            }
            op if op & 0xC7 == 0x06 => {
                let dst = (op >> 3) & 0x07;
                let value = self.fetch8(bus)?;
                self.write_r8(dst, value, bus)?;
                if dst == 6 {
                    12
                } else {
                    8
                }
            }
            op if op & 0xC7 == 0x04 => {
                let code = (op >> 3) & 0x07;
                let value = self.read_r8(code, bus)?;
                let result = self.increment8(value);
                self.write_r8(code, result, bus)?;
                if code == 6 {
                    12
                } else {
                    4
                }
            }
            op if op & 0xC7 == 0x05 => {
                let code = (op >> 3) & 0x07;
                let value = self.read_r8(code, bus)?;
                let result = self.decrement8(value);
                self.write_r8(code, result, bus)?;
                if code == 6 {
                    12
                } else {
                    4
                }
            }
            0x40..=0x75 | 0x77..=0x7F => {
                let src = opcode & 7;
                let dst = (opcode >> 3) & 7;
                let value = self.read_r8(src, bus)?;
                self.write_r8(dst, value, bus)?;
                if src == 6 || dst == 6 {
                    8
                } else {
                    4
                }
            }
            0x80..=0xBF => {
                let source = opcode & 0x07;
                let value = self.read_r8(source, bus)?;
                self.alu_a((opcode >> 3) & 0x07, value);
                if source == 6 {
                    8
                } else {
                    4
                }
            }
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let value = self.fetch8(bus)?;
                self.alu_a((opcode >> 3) & 0x07, value);
                8
            }
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
            0xE8 | 0xF8 => {
                let displacement = self.fetch8(bus)? as i8;
                let result = self.sp_plus_signed(displacement);
                if opcode == 0xE8 {
                    self.registers.sp = result;
                    16
                } else {
                    self.registers.set_hl(result);
                    12
                }
            }
            0x08 => {
                let address = self.fetch16(bus)?;
                let [low, high] = self.registers.sp.to_le_bytes();
                if !bus.write(address, low) {
                    return Err(CpuError::UnmappedWrite { address });
                }
                let next = address.wrapping_add(1);
                if !bus.write(next, high) {
                    return Err(CpuError::UnmappedWrite { address: next });
                }
                20
            }
            0xC1 | 0xD1 | 0xE1 | 0xF1 => {
                let pair = (opcode >> 4) & 0x03;
                let value = self.pop16(bus)?;
                self.set_stack_pair(pair, value);
                12
            }
            0xC5 | 0xD5 | 0xE5 | 0xF5 => {
                let pair = (opcode >> 4) & 0x03;
                let value = self.stack_pair(pair);
                self.push16(bus, value)?;
                16
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
            0xF9 => {
                self.registers.sp = self.registers.hl();
                8
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
            _ => {
                self.registers.pc = opcode_address;
                return Err(CpuError::UnsupportedOpcode {
                    address: opcode_address,
                    opcode,
                });
            }
        };
        Ok(cycles)
    }
}
