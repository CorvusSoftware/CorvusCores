// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 accumulator, arithmetic, and flag operations.
//!
//! Z, N, H, and C correspond to bits 7, 6, 5, and 4 of F.
//! All arithmetic wraps at the register width.

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

    /// Execute the 8 accumulator ALU families, encoded in bits 5..3.
    ///
    /// The register operand and immediate forms use the same family field:
    /// ADD, ADC, SUB, SBC, AND, XOR, OR, CP.
    pub(super) fn alu_a(&mut self, family: u8, value: u8) {
        match family {
            0 => self.add_to_a(value, false),
            1 => self.add_to_a(value, true),
            2 => self.sub_from_a(value, false, true),
            3 => self.sub_from_a(value, true, true),
            4 => {
                self.registers.a &= value;
                self.registers
                    .set_flags(FLAG_HALF_CARRY | if self.registers.a == 0 { FLAG_ZERO } else { 0 });
            }
            5 => {
                self.registers.a ^= value;
                self.registers
                    .set_flags(if self.registers.a == 0 { FLAG_ZERO } else { 0 });
            }
            6 => {
                self.registers.a |= value;
                self.registers
                    .set_flags(if self.registers.a == 0 { FLAG_ZERO } else { 0 });
            }
            7 => self.sub_from_a(value, false, false), // CP: compare without storing
            _ => unreachable!("ALU family field is three bits"),
        }
    }

    fn add_to_a(&mut self, value: u8, with_carry: bool) {
        let a = self.registers.a;
        let carry = u8::from(with_carry && self.registers.flags() & FLAG_CARRY != 0);
        let sum = u16::from(a) + u16::from(value) + u16::from(carry);
        let result = sum as u8;
        let mut flags = 0;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if (a & 0x0F) + (value & 0x0F) + carry > 0x0F {
            flags |= FLAG_HALF_CARRY;
        }
        if sum > 0xFF {
            flags |= FLAG_CARRY;
        }
        self.registers.a = result;
        self.registers.set_flags(flags);
    }

    fn sub_from_a(&mut self, value: u8, with_carry: bool, store: bool) {
        let a = self.registers.a;
        let carry = u8::from(with_carry && self.registers.flags() & FLAG_CARRY != 0);
        let subtrahend = u16::from(value) + u16::from(carry);
        let result = u16::from(a).wrapping_sub(subtrahend) as u8;
        let mut flags = FLAG_SUBTRACT;
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if (a & 0x0F) < (value & 0x0F) + carry {
            flags |= FLAG_HALF_CARRY;
        }
        if u16::from(a) < subtrahend {
            flags |= FLAG_CARRY;
        }
        if store {
            self.registers.a = result;
        }
        self.registers.set_flags(flags);
    }

    /// ADD HL,r16: preserve Z, clear N, set H from bit 11 and C from bit 15.
    pub(super) fn add_hl(&mut self, value: u16) {
        let hl = self.registers.hl();
        let (result, carry) = hl.overflowing_add(value);
        let mut flags = self.registers.flags() & FLAG_ZERO;
        if (hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF {
            flags |= FLAG_HALF_CARRY;
        }
        if carry {
            flags |= FLAG_CARRY;
        }
        self.registers.set_hl(result);
        self.registers.set_flags(flags);
    }

    /// ADD SP,e8 and LD HL,SP+e8 share the same H/C calculation.
    /// Flags reflect addition of the sign-extended displacement's low byte.
    pub(super) fn sp_plus_signed(&mut self, displacement: i8) -> u16 {
        let sp = self.registers.sp;
        let offset = displacement as i16 as u16;
        let result = sp.wrapping_add(offset);
        let mut flags = 0;
        if (sp & 0x000F) + (offset & 0x000F) > 0x000F {
            flags |= FLAG_HALF_CARRY;
        }
        if (sp & 0x00FF) + (offset & 0x00FF) > 0x00FF {
            flags |= FLAG_CARRY;
        }
        self.registers.set_flags(flags);
        result
    }

    /// DAA converts the result of binary addition/subtraction to packed BCD.
    pub(super) fn daa(&mut self) {
        let old_flags = self.registers.flags();
        let subtract = old_flags & FLAG_SUBTRACT != 0;
        let mut carry = old_flags & FLAG_CARRY != 0;
        let mut adjustment = 0_u8;
        let a = self.registers.a;

        if old_flags & FLAG_HALF_CARRY != 0 || (!subtract && a & 0x0F > 9) {
            adjustment |= 0x06;
        }
        if carry || (!subtract && a > 0x99) {
            adjustment |= 0x60;
            carry = true;
        }

        let result = if subtract {
            a.wrapping_sub(adjustment)
        } else {
            a.wrapping_add(adjustment)
        };
        self.registers.a = result;
        let mut flags = old_flags & FLAG_SUBTRACT; // N remains unchanged.
        if result == 0 {
            flags |= FLAG_ZERO;
        }
        if carry {
            flags |= FLAG_CARRY;
        }
        self.registers.set_flags(flags);
    }

    pub(super) fn complement_a(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers
            .set_flags(self.registers.flags() | FLAG_SUBTRACT | FLAG_HALF_CARRY);
    }

    pub(super) fn set_carry(&mut self) {
        self.registers
            .set_flags((self.registers.flags() & FLAG_ZERO) | FLAG_CARRY);
    }

    pub(super) fn complement_carry(&mut self) {
        let flags = self.registers.flags();
        self.registers
            .set_flags((flags & FLAG_ZERO) | ((flags ^ FLAG_CARRY) & FLAG_CARRY));
    }

    /// RLCA, RRCA, RLA and RRA always clear Z (unlike CB-prefixed rotates).
    pub(super) fn rotate_accumulator(&mut self, opcode: u8) {
        let a = self.registers.a;
        let carry_in = u8::from(self.registers.flags() & FLAG_CARRY != 0);
        let (result, carry_out) = match opcode {
            0x07 => (a.rotate_left(1), a & 0x80 != 0),
            0x0F => (a.rotate_right(1), a & 0x01 != 0),
            0x17 => ((a << 1) | carry_in, a & 0x80 != 0),
            0x1F => ((a >> 1) | (carry_in << 7), a & 0x01 != 0),
            _ => unreachable!("not an accumulator rotation opcode"),
        };
        self.registers.a = result;
        self.registers
            .set_flags(if carry_out { FLAG_CARRY } else { 0 });
    }
}
