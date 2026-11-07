// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! An intentionally small SM83 interpreter with nominal T-cycle counts.
//!
//! The bus is not cycle-accurate yet; the returned count is instruction
//! timing only, not proof that the surrounding machine has advanced.

use super::{
    Cpu, CpuError, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO,
};
use crate::bus::Bus;

impl Cpu {
    fn fetch8(&mut self, bus: &Bus) -> Result<u8, CpuError> {
        let address = self.registers.pc;
        let value = bus.read(address).ok_or(CpuError::UnmappedRead { address })?;
        self.registers.pc = address.wrapping_add(1);
        Ok(value)
    }

    fn fetch16(&mut self, bus: &Bus) -> Result<u16, CpuError> {
        let low = self.fetch8(bus)?;
        let high = self.fetch8(bus)?;
        Ok(u16::from_le_bytes([low, high]))
    }

    fn read_r8(&self, code: u8, bus: &Bus) -> Result<u8, CpuError> {
        let r = &self.registers;
        match code {
            0 => Ok(r.b),
            1 => Ok(r.c),
            2 => Ok(r.d),
            3 => Ok(r.e),
            4 => Ok(r.h),
            5 => Ok(r.l),
            6 => bus.read(r.hl()).ok_or(CpuError::UnmappedRead { address: r.hl() }),
            7 => Ok(r.a),
            _ => unreachable!("8-bit register field is three bits"),
        }
    }

    fn write_r8(&mut self, code: u8, value: u8, bus: &mut Bus) -> Result<(), CpuError> {
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
        match opcode {
            0x20 | 0xC2 => flags & FLAG_ZERO == 0,
            0x28 | 0xCA => flags & FLAG_ZERO != 0,
            0x30 | 0xD2 => flags & FLAG_CARRY == 0,
            0x38 | 0xDA => flags & FLAG_CARRY != 0,
            _ => unreachable!("not a conditional jump opcode"),
        }
    }

    fn increment8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        let mut flags = self.registers.flags() & FLAG_CARRY;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if value & 0x0F == 0x0F {
            flags |= FLAG_HALF_CARRY;
        }
        self.registers.set_flags(flags);
        result
    }

    fn decrement8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        let mut flags = (self.registers.flags() & FLAG_CARRY) | FLAG_SUBTRACT;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if value & 0x0F == 0 {
            flags |= FLAG_HALF_CARRY;
        }
        self.registers.set_flags(flags);
        result
    }

    fn add_a(&mut self, value: u8) {
        let a = self.registers.a;
        let (result, carry) = a.overflowing_add(value);
        self.registers.a = result;
        let mut flags = 0;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if (a & 0x0F) + (value & 0x0F) > 0x0F {
            flags |= FLAG_HALF_CARRY;
        }
        if carry {
            flags |= FLAG_CARRY;
        }
        self.registers.set_flags(flags);
    }

    fn sub_a(&mut self, value: u8, store: bool) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        if store {
            self.registers.a = result;
        }
        let mut flags = FLAG_SUBTRACT;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if (a & 0x0F) < (value & 0x0F) {
            flags |= FLAG_HALF_CARRY;
        }
        if a < value {
            flags |= FLAG_CARRY;
        }
        self.registers.set_flags(flags);
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
                if dst == 6 { 12 } else { 8 }
            }
            op if op & 0xC7 == 0x04 => {
                let code = (op >> 3) & 0x07;
                let value = self.read_r8(code, bus)?;
                let result = self.increment8(value);
                self.write_r8(code, result, bus)?;
                if code == 6 { 12 } else { 4 }
            }
            op if op & 0xC7 == 0x05 => {
                let code = (op >> 3) & 0x07;
                let value = self.read_r8(code, bus)?;
                let result = self.decrement8(value);
                self.write_r8(code, result, bus)?;
                if code == 6 { 12 } else { 4 }
            }
            0x40..=0x75 | 0x77..=0x7F => {
                let src = opcode & 7;
                let dst = (opcode >> 3) & 7;
                let value = self.read_r8(src, bus)?;
                self.write_r8(dst, value, bus)?;
                if src == 6 || dst == 6 { 8 } else { 4 }
            }
            0x80..=0x87 => {
                let source = opcode & 7;
                let value = self.read_r8(source, bus)?;
                self.add_a(value);
                if source == 6 { 8 } else { 4 }
            }
            0x90..=0x97 => {
                let source = opcode & 7;
                let value = self.read_r8(source, bus)?;
                self.sub_a(value, true);
                if source == 6 { 8 } else { 4 }
            }
            0xA8..=0xAF => {
                let source = opcode & 7;
                let value = self.read_r8(source, bus)?;
                self.registers.a ^= value;
                self.registers.set_flags(if self.registers.a == 0 { FLAG_ZERO } else { 0 });
                if source == 6 { 8 } else { 4 }
            }
            0xB8..=0xBF => {
                let source = opcode & 7;
                let value = self.read_r8(source, bus)?;
                self.sub_a(value, false);
                if source == 6 { 8 } else { 4 }
            }
            0xC6 => {
                let value = self.fetch8(bus)?;
                self.add_a(value);
                8
            }
            0xD6 => {
                let value = self.fetch8(bus)?;
                self.sub_a(value, true);
                8
            }
            0xEE => {
                let value = self.fetch8(bus)?;
                self.registers.a ^= value;
                self.registers.set_flags(if self.registers.a == 0 { FLAG_ZERO } else { 0 });
                8
            }
            0xFE => {
                let value = self.fetch8(bus)?;
                self.sub_a(value, false);
                8
            }
            0x18 => {
                let displacement = self.fetch8(bus)? as i8;
                self.registers.pc =
                    self.registers.pc.wrapping_add(i16::from(displacement) as u16);
                12
            }
            0x20 | 0x28 | 0x30 | 0x38 => {
                let displacement = self.fetch8(bus)? as i8;
                if self.check_condition(opcode) {
                    self.registers.pc =
                        self.registers.pc.wrapping_add(i16::from(displacement) as u16);
                    12
                } else {
                    8
                }
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
            0xEA => {
                let address = self.fetch16(bus)?;
                if !bus.write(address, self.registers.a) {
                    return Err(CpuError::UnmappedWrite { address });
                }
                16
            }
            0xFA => {
                let address = self.fetch16(bus)?;
                self.registers.a =
                    bus.read(address).ok_or(CpuError::UnmappedRead { address })?;
                16
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
