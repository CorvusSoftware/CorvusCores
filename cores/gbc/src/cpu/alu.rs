// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Arithmetic and flag logic for the implemented SM83 instruction subset.

use super::{Cpu, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

impl Cpu {
    pub(super) fn increment8(&mut self, value: u8) -> u8 {
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

    pub(super) fn decrement8(&mut self, value: u8) -> u8 {
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

    pub(super) fn add_a(&mut self, value: u8) {
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

    pub(super) fn sub_a(&mut self, value: u8, store: bool) {
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
}
