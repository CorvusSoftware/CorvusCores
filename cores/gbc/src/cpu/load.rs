// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 accumulator-to-memory transfers.
//!
//! The untimed bus returns errors for unimplemented I/O registers. On a failed
//! memory access, A and HL stay unchanged; the instruction's fetched immediate
//! bytes may already have advanced PC.

use super::{Cpu, CpuError};
use crate::bus::Bus;

fn read_memory(bus: &Bus, address: u16) -> Result<u8, CpuError> {
    bus.read(address).ok_or(CpuError::UnmappedRead { address })
}

fn write_memory(bus: &mut Bus, address: u16, value: u8) -> Result<(), CpuError> {
    if bus.write(address, value) {
        Ok(())
    } else {
        Err(CpuError::UnmappedWrite { address })
    }
}

impl Cpu {
    /// Execute the remaining register-indirect, auto-indexed and high-memory
    /// transfer forms, returning their nominal number of T-cycles.
    pub(super) fn execute_transfer(&mut self, opcode: u8, bus: &mut Bus) -> Result<u8, CpuError> {
        match opcode {
            0x02 | 0x12 => {
                let address = if opcode == 0x02 {
                    self.registers.bc()
                } else {
                    self.registers.de()
                };
                write_memory(bus, address, self.registers.a)?;
                Ok(8)
            }
            0x0A | 0x1A => {
                let address = if opcode == 0x0A {
                    self.registers.bc()
                } else {
                    self.registers.de()
                };
                self.registers.a = read_memory(bus, address)?;
                Ok(8)
            }
            0x22 | 0x32 => {
                let address = self.registers.hl();
                write_memory(bus, address, self.registers.a)?;
                let next = if opcode == 0x22 {
                    address.wrapping_add(1)
                } else {
                    address.wrapping_sub(1)
                };
                self.registers.set_hl(next);
                Ok(8)
            }
            0x2A | 0x3A => {
                let address = self.registers.hl();
                let value = read_memory(bus, address)?;
                let next = if opcode == 0x2A {
                    address.wrapping_add(1)
                } else {
                    address.wrapping_sub(1)
                };
                self.registers.a = value;
                self.registers.set_hl(next);
                Ok(8)
            }
            0xE0 | 0xF0 => {
                let offset = self.fetch8(bus)?;
                let address = 0xFF00 | u16::from(offset);
                if opcode == 0xE0 {
                    write_memory(bus, address, self.registers.a)?;
                } else {
                    self.registers.a = read_memory(bus, address)?;
                }
                Ok(12)
            }
            0xE2 | 0xF2 => {
                let address = 0xFF00 | u16::from(self.registers.c);
                if opcode == 0xE2 {
                    write_memory(bus, address, self.registers.a)?;
                } else {
                    self.registers.a = read_memory(bus, address)?;
                }
                Ok(8)
            }
            0xEA | 0xFA => {
                let address = self.fetch16(bus)?;
                if opcode == 0xEA {
                    write_memory(bus, address, self.registers.a)?;
                } else {
                    self.registers.a = read_memory(bus, address)?;
                }
                Ok(16)
            }
            _ => unreachable!("not a transfer opcode"),
        }
    }
}
