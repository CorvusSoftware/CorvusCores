// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Partial SM83 CPU interpreter.
//!
//! Interrupt service and HALT are modeled at instruction boundaries.
//! No boot ROM, STOP, or cycle-accurate bus/peripheral scheduling exists.

mod alu;

#[cfg(test)]
mod alu_tests;
mod cb;
mod control_flow;
mod error;
mod execute;
mod interrupts;
mod load;

#[cfg(test)]
mod interrupt_tests;
#[cfg(test)]
mod load_tests;
mod registers;
mod stack;

#[cfg(test)]
mod stack_tests;

#[cfg(test)]
mod cb_tests;
#[cfg(test)]
mod control_flow_tests;
#[cfg(test)]
mod tests;

pub use error::CpuError;
pub use registers::{Registers, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

/// A CPU with explicitly initialized state; not a hardware power-on snapshot.
#[derive(Clone, Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
    ime: bool,
    ime_enable_pending: bool,
    halted: bool,
    halt_bug: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }
}
