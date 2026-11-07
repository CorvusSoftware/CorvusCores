// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy Color core foundations.
//!
//! Cartridge parsing, memory, partial SM83 execution, timer and interrupt
//! registers are implemented. CPU interrupt service, a complete scheduler,
//! PPU timing, boot and CorvusSoftware API integration are not.

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod interrupts;
mod memory;
pub mod timer;
mod video;
