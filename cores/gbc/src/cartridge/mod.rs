// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cartridge header decoding and mapper implementations.

mod mapper;
mod error;
mod header;
mod mbc1;
#[cfg(test)]
mod mbc1_tests;
mod rom_only;

#[cfg(test)]
mod tests;

pub use cartridge::Cartridge;
pub use error::CartridgeError;
pub use header::{CartridgeHeader, CgbSupport};
pub use mbc1::Mbc1Cartridge;
pub use rom_only::RomOnlyCartridge;
