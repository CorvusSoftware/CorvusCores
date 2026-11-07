// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SM83 interrupt master enable, interrupt entry, and HALT behavior.
//!
//! The bus models IF/IE. This module owns CPU state, including the one-
//! instruction EI delay and the HALT bug. The returned 20 T-cycles for
//! interrupt entry are nominal: bus accesses are still untimed.

use super::{Cpu, CpuError};
use crate::bus::Bus;

impl Cpu {
    /// IME is CPU-internal, not a readable memory-mapped register.
    pub fn interrupts_enabled(&self) -> bool {
        self.ime
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Service one enabled request before fetching an instruction, if IME
    /// permits it. HALT wakes on any enabled request, even with IME disabled.
    ///
    /// If a stack access fails, IF is not acknowledged and IME is preserved.
    /// The partial stack side effects match the existing bus error contract.
    pub(super) fn service_interrupt(&mut self, bus: &mut Bus) -> Result<Option<u8>, CpuError> {
        let Some(interrupt) = bus.highest_pending_interrupt() else {
            return Ok(None);
        };

        self.halted = false;
        if !self.ime {
            return Ok(None);
        }

        // EI immediately followed by HALT with a pending interrupt triggers
        // the HALT bug before delayed IME becomes active. The return address
        // must point back to HALT, not to its successor.
        let return_address = if self.halt_bug {
            self.registers.pc.wrapping_sub(1)
        } else {
            self.registers.pc
        };
        self.push16(bus, return_address)?;

        self.ime = false;
        self.ime_enable_pending = false;
        self.halt_bug = false;
        bus.acknowledge_interrupt(interrupt);
        self.registers.pc = interrupt.vector();
        Ok(Some(20))
    }

    pub(super) fn execute_halt(&mut self, bus: &Bus) {
        if !self.ime && bus.highest_pending_interrupt().is_some() {
            // The next instruction's opcode fetch does not increment PC.
            self.halt_bug = true;
        } else {
            self.halted = true;
        }
    }

    /// Apply a pending EI after the *following* completed instruction.
    /// DI overrides and cancels the scheduled enable immediately.
    pub(super) fn finish_ime_delay(&mut self, scheduled_before: bool, opcode: u8) {
        if scheduled_before && opcode != 0xF3 {
            self.ime = true;
            if opcode != 0xFB {
                self.ime_enable_pending = false;
            }
        }
    }
}
