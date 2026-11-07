// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy Color core foundations.
//!
//! Cartridge parsing, memory, and a partial SM83 interpreter are implemented.
//! PPU timing, peripherals, boot, and CorvusSoftware API integration are not.

pub mod bus;
pub mod cartridge;
pub mod cpu;
mod memory;
mod video;
