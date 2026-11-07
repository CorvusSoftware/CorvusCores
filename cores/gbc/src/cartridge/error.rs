// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Errors returned by cartridge parsing and currently supported mapper loaders.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CartridgeError {
    HeaderTooShort { actual: usize },
    UnsupportedCartridgeType { code: u8 },
    UnsupportedRomSizeCode { code: u8 },
    UnsupportedRamSizeCode { code: u8 },
    IncorrectRomOnlyLength { actual: usize },
}

impl fmt::Display for CartridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTooShort { actual } => {
                write!(f, "cartridge header is too short: {actual} bytes")
            }
            Self::UnsupportedCartridgeType { code } => {
                write!(f, "cartridge type 0x{code:02X} is not supported yet")
            }
            Self::UnsupportedRomSizeCode { code } => {
                write!(f, "ROM size code 0x{code:02X} is not supported by ROM-only cartridges")
            }
            Self::UnsupportedRamSizeCode { code } => {
                write!(f, "RAM size code 0x{code:02X} is not supported by ROM-only cartridges")
            }
            Self::IncorrectRomOnlyLength { actual } => {
                write!(f, "expected a 32768-byte ROM-only cartridge, got {actual} bytes")
            }
        }
    }
}

impl std::error::Error for CartridgeError {}
