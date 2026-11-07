// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! An intentionally small SM83 interpreter with nominal T-cycle counts.
//!
//! The bus is not cycle-accurate yet; the returned count is instruction
//! timing only, not proof that the surrounding machine has advanced.

use super::{Cpu, CpuError};
use crate::bus::Bus;

impl Cpu {
    pub(super) fn fetch8(&mut self, bus: &Bus) -> Result<u8, CpuError> {
        let address = self.registers.pc;
        let value = bus
            .read(address)
            .ok_or(CpuError::UnmappedRead { address })?;
        if self.halt_bug {
            self.halt_bug = false;
        } else {
            self.registers.pc = address.wrapping_add(1);
        }
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

    /// Execute one supported instruction, interrupt entry, or HALT idle step.
    ///
    /// Returns nominal T-cycles, without advancing bus peripherals. On
    /// unmapped memory errors, PC, SP or memory may already be modified.
    /// Unsupported opcodes restore the PC to the opcode address.
    pub fn step(&mut self, bus: &mut Bus) -> Result<u8, CpuError> {
        if let Some(cycles) = self.service_interrupt(bus)? {
            return Ok(cycles);
        }
        if self.halted {
            return Ok(4);
        }

        let scheduled_before = self.ime_enable_pending;
        let opcode_address = self.registers.pc;
        let opcode = self.fetch8(bus)?;

        let cycles = match opcode {
            0x00 => 4, // NOP
            0x76 => {
                self.execute_halt(bus);
                4
            }
            0xF3 => {
                self.ime = false;
                self.ime_enable_pending = false;
                4
            }
            0xFB => {
                self.ime_enable_pending = true;
                4
            }
            0xD9 => {
                let address = self.pop16(bus)?;
                self.registers.pc = address;
                self.ime = true;
                self.ime_enable_pending = false;
                16
            }
            0xCB => {
                let cb_opcode = self.fetch8(bus)?;
                self.execute_cb(cb_opcode, bus)?
            }
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0xC0 | 0xC2 | 0xC3 | 0xC4 | 0xC7 | 0xC8
            | 0xC9 | 0xCA | 0xCC | 0xCD | 0xCF | 0xD0 | 0xD2 | 0xD4 | 0xD7 | 0xD8
            | 0xDA | 0xDC | 0xDF | 0xE7 | 0xE9 | 0xEF | 0xF7 | 0xFF => {
                self.execute_control_flow(opcode, bus)?
            }
            0x02 | 0x12 | 0x0A | 0x1A | 0x22 | 0x32 | 0x2A | 0x3A | 0xE0 | 0xF0 | 0xE2 | 0xF2
            | 0xEA | 0xFA => self.execute_transfer(opcode, bus)?,
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
            0xF9 => {
                self.registers.sp = self.registers.hl();
                8
            }
            _ => {
                self.registers.pc = opcode_address;
                return Err(CpuError::UnsupportedOpcode {
                    address: opcode_address,
                    opcode,
                });
            }
        };
        self.finish_ime_delay(scheduled_before, opcode);
        Ok(cycles)
    }
}
