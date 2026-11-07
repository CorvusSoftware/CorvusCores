// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cartridge header decoding and mapper implementations.

mod error;
mod header;
mod mapper;
mod mbc1;
#[cfg(test)]
mod mbc1_tests;
mod mbc3;
#[cfg(test)]
mod mbc3_tests;
mod rom_only;
mod rtc;
#[cfg(test)]
mod rtc_tests;

#[cfg(test)]
mod tests;

pub use error::CartridgeError;
pub use header::{CartridgeHeader, CgbSupport};
pub use mapper::Cartridge;
pub use mbc1::Mbc1Cartridge;
pub use mbc3::Mbc3Cartridge;
pub use rom_only::RomOnlyCartridge;
