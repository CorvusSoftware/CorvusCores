// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Partial SM83 CPU interpreter.
//!
//! No boot ROM sequence, interrupts, HALT, STOP, or timing of bus peripherals.
//! Only instructions implemented in `execute` may be stepped.

mod alu;

#[cfg(test)]
mod alu_tests;
mod cb;
mod error;
mod execute;
mod load;

#[cfg(test)]
mod load_tests;
mod registers;
mod stack;

#[cfg(test)]
mod stack_tests;

#[cfg(test)]
mod cb_tests;
#[cfg(test)]
mod tests;

pub use error::CpuError;
pub use registers::{Registers, FLAG_CARRY, FLAG_HALF_CARRY, FLAG_SUBTRACT, FLAG_ZERO};

/// A CPU with explicitly initialized state; not a hardware power-on snapshot.
#[derive(Clone, Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }
}
