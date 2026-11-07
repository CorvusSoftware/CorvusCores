// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy Color core foundations.
//!
//! Cartridge parsing, memory, partial SM83 execution, timers and
//! interrupts have initial implementations. The machine runner advances the
//! timer at instruction boundaries, not at individual bus accesses.
//! PPU timing, boot and CorvusSoftware API integration are not implemented.

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod interrupts;
pub mod machine;
mod memory;
pub mod timer;
mod video;
