// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 register file, with the low nibble of F permanently masked off.

use serde::{Deserialize, Serialize};

pub const FLAG_ZERO: u8 = 0x80;
pub const FLAG_SUBTRACT: u8 = 0x40;
pub const FLAG_HALF_CARRY: u8 = 0x20;
pub const FLAG_CARRY: u8 = 0x10;

/// Default is an all-zero test state, not a DMG/CGB post-boot register state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn flags(&self) -> u8 {
        self.f
    }

    pub fn set_flags(&mut self, value: u8) {
        self.f = value & 0xF0;
    }

    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f])
    }

    pub fn set_af(&mut self, value: u16) {
        let [a, f] = value.to_be_bytes();
        self.a = a;
        self.set_flags(f);
    }

    pub fn bc(&self) -> u16 {
        u16::from_be_bytes([self.b, self.c])
    }

    pub fn set_bc(&mut self, value: u16) {
        [self.b, self.c] = value.to_be_bytes();
    }

    pub fn de(&self) -> u16 {
        u16::from_be_bytes([self.d, self.e])
    }

    pub fn set_de(&mut self, value: u16) {
        [self.d, self.e] = value.to_be_bytes();
    }

    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_hl(&mut self, value: u16) {
        [self.h, self.l] = value.to_be_bytes();
    }

    pub(crate) fn read_pair(&self, code: u8) -> u16 {
        match code {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => unreachable!("16-bit register field is only two bits"),
        }
    }

    pub(crate) fn write_pair(&mut self, code: u8, value: u16) {
        match code {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => self.sp = value,
            _ => unreachable!("16-bit register field is only two bits"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_unusable_flag_bits() {
        let mut registers = Registers::default();
        registers.set_flags(0xFF);
        assert_eq!(registers.flags(), 0xF0);
        registers.set_af(0x123F);
        assert_eq!(registers.af(), 0x1230);
    }

    #[test]
    fn sixteen_bit_pairs_round_trip_in_big_endian_register_order() {
        let mut registers = Registers::default();
        registers.set_bc(0x1234);
        registers.set_de(0x5678);
        registers.set_hl(0x9ABC);
        assert_eq!((registers.b, registers.c), (0x12, 0x34));
        assert_eq!(registers.bc(), 0x1234);
        assert_eq!(registers.de(), 0x5678);
        assert_eq!(registers.hl(), 0x9ABC);
    }
}
