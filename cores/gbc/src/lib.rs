//! Game Boy Color core foundations.
//!
//! Cartridge parsing and selected memory regions are implemented. CPU/PPU
//! timing, peripherals, boot and CorvusSoftware API integration are not.

pub mod bus;
pub mod cartridge;
mod memory;
mod video;
