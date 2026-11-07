// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cartridge header decoding and mapper implementations.

mod error;
mod header;
mod rom_only;

#[cfg(test)]
mod tests;

pub use error::CartridgeError;
pub use header::{CartridgeHeader, CgbSupport};
pub use rom_only::RomOnlyCartridge;
