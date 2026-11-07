// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit CPU failures while the bus and opcode coverage are incomplete.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuError {
    UnmappedRead { address: u16 },
    UnmappedWrite { address: u16 },
    UnsupportedOpcode { address: u16, opcode: u8 },
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmappedRead { address } => {
                write!(f, "CPU read from unimplemented address 0x{address:04X}")
            }
            Self::UnmappedWrite { address } => {
                write!(f, "CPU write to unimplemented address 0x{address:04X}")
            }
            Self::UnsupportedOpcode { address, opcode } => {
                write!(f, "opcode 0x{opcode:02X} at 0x{address:04X} is not implemented")
            }
        }
    }
}

impl std::error::Error for CpuError {}
