// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Owns CPU, bus and nominal time for the incomplete GBC emulator.
//!
//! The scheduler selects CPU work or a VRAM DMA stall, advances peripherals
//! at instruction boundaries and accounts for the two sources separately.
//! This is not cycle-accurate bus arbitration or the CorvusSoftware frontend API.

use crate::bus::Bus;
use crate::cartridge::RomOnlyCartridge;
use crate::cpu::Cpu;

mod scheduler;

#[cfg(test)]
mod scheduler_tests;
#[cfg(test)]
mod tests;

/// Owns the CPU and bus so a completed CPU step clocks peripherals once.
pub struct Machine {
    cpu: Cpu,
    bus: Bus,
    elapsed_t_cycles: u64,
    dma_stall_t_cycles: u64,
}

impl Machine {
    /// Start from deterministic zero-initialized test state, not a boot ROM.
    pub fn new(cartridge: RomOnlyCartridge) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(cartridge),
            elapsed_t_cycles: 0,
            dma_stall_t_cycles: 0,
        }
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    /// Set CPU registers for experiments until boot initialization exists.
    pub fn cpu_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Configure the partial bus or inject an interrupt. External manual
    /// calls to Bus::tick_t_cycles do not update elapsed_t_cycles.
    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    /// Total nominal T-cycles from successful Machine::step calls.
    ///
    /// Includes CPU steps (instructions, HALT idle and interrupt entry) and
    /// VRAM DMA pauses. Manual Bus::tick_t_cycles calls are not counted.
    pub fn elapsed_t_cycles(&self) -> u64 {
        self.elapsed_t_cycles
    }

    /// Cycles spent on CPU steps, including HALT idle and interrupt entry.
    ///
    /// This is not a cycle-accurate measure of individual CPU bus accesses.
    pub fn cpu_t_cycles(&self) -> u64 {
        self.elapsed_t_cycles - self.dma_stall_t_cycles
    }

    /// Cycles spent waiting for GDMA or active HBlank DMA blocks.
    ///
    /// Does not include OAM DMA, which currently does not stall CPU execution
    /// in the partial bus model.
    pub fn dma_stall_t_cycles(&self) -> u64 {
        self.dma_stall_t_cycles
    }
}
