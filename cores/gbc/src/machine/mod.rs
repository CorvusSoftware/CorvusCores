// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Instruction-boundary orchestration for the incomplete GBC emulator.
//!
//! CPU instructions, HALT idle steps and interrupt entry return nominal
//! CPU T-cycles. The runner advances timer, OAM/VRAM DMA and the nominal
//! LCD PPU after each CPU step. GDMA and active HDMA blocks hold CPU fetches. It does not interleave memory
//! accesses with peripherals and is not a cycle-accurate scheduler or
//! the CorvusSoftware frontend API.

use crate::bus::Bus;
use crate::cartridge::RomOnlyCartridge;
use crate::cpu::{Cpu, CpuError};

#[cfg(test)]
mod tests;

/// Owns the CPU and bus so a completed CPU step clocks peripherals once.
pub struct Machine {
    cpu: Cpu,
    bus: Bus,
    elapsed_t_cycles: u64,
}

impl Machine {
    /// Start from deterministic zero-initialized test state, not a boot ROM.
    pub fn new(cartridge: RomOnlyCartridge) -> Self {
        Self {
            cpu: Cpu::new(),
            bus: Bus::new(cartridge),
            elapsed_t_cycles: 0,
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

    /// Elapsed normal-speed T-cycles from instructions and DMA stall steps.
    pub fn elapsed_t_cycles(&self) -> u64 {
        self.elapsed_t_cycles
    }

    /// Execute one CPU step or spend four T-cycles stalled by VRAM DMA,
    /// then advance timers, DMA, LCD modes and IRQs.
    ///
    /// The next step observes any interrupt generated in this step. If
    /// execution fails, no time is advanced or accounted, although existing
    /// CPU and bus operations may already have changed state.
    ///
    /// This remains *instruction-boundary* timing: writes to timer/LCD/DMA
    /// registers take effect before the step's T-cycles are advanced, not
    /// at their real per-M-cycle location.
    pub fn step(&mut self) -> Result<u8, CpuError> {
        // A GDMA transfer, or the current HDMA block, suspends CPU fetches.
        // The bus continues clocking the DMA, PPU and DIV/TIMA. Four cycles
        // is a host-runner scheduling quantum, not a CPU instruction.
        let cycles = if self.bus.vram_dma_cpu_stalled() {
            4
        } else {
            self.cpu.step(&mut self.bus)?
        };
        self.bus.tick_t_cycles(u32::from(cycles));
        self.elapsed_t_cycles += u64::from(cycles);
        Ok(cycles)
    }
}
