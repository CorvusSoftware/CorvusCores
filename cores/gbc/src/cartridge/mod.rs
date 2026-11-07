//! Cartridge header decoding and mapper implementations.

mod header;
mod rom_only;

#[cfg(test)]
mod tests;

pub use header::{CartridgeError, CartridgeHeader, CgbSupport};
pub use rom_only::RomOnlyCartridge;
