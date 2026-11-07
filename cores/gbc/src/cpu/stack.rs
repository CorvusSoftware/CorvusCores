// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 stack operations, using the current untimed bus.
//!
//! PUSH writes the high byte first at SP-1, then the low byte at SP-2.
//! POP reads the low byte first. SP wraps in 16 bits.
//! Bus errors remain explicit; a multi-byte access may have partially
//! changed SP or memory when the later access fails.

use super::{Cpu, CpuError};
use crate::bus::Bus;

impl Cpu {
    pub(super) fn push16(&mut self, bus: &mut Bus, value: u16) -> Result<(), CpuError> {
        let [high, low] = value.to_be_bytes();
        for byte in [high, low] {
            self.registers.sp = self.registers.sp.wrapping_sub(1);
            let address = self.registers.sp;
            if !bus.write(address, byte) {
                return Err(CpuError::UnmappedWrite { address });
            }
        }
        Ok(())
    }

    pub(super) fn pop16(&mut self, bus: &Bus) -> Result<u16, CpuError> {
        let low_address = self.registers.sp;
        let low = bus
            .read(low_address)
            .ok_or(CpuError::UnmappedRead { address: low_address })?;
        self.registers.sp = low_address.wrapping_add(1);

        let high_address = self.registers.sp;
        let high = bus
            .read(high_address)
            .ok_or(CpuError::UnmappedRead { address: high_address })?;
        self.registers.sp = high_address.wrapping_add(1);

        Ok(u16::from_le_bytes([low, high]))
    }

    pub(super) fn stack_pair(&self, code: u8) -> u16 {
        match code {
            0 => self.registers.bc(),
            1 => self.registers.de(),
            2 => self.registers.hl(),
            3 => self.registers.af(),
            _ => unreachable!("stack register field is two bits"),
        }
    }

    pub(super) fn set_stack_pair(&mut self, code: u8, value: u16) {
        match code {
            0 => self.registers.set_bc(value),
            1 => self.registers.set_de(value),
            2 => self.registers.set_hl(value),
            3 => self.registers.set_af(value),
            _ => unreachable!("stack register field is two bits"),
        }
    }
}
