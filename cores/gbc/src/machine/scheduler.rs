// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Instruction-boundary CPU dispatch, VRAM DMA stalls and cycle accounting.
//!
//! CPU instructions, interrupt entry and HALT idle each produce nominal
//! T-cycles. GDMA and active HDMA blocks instead consume four T-cycles
//! without fetching an opcode. In both cases the bus advances *after* the
//! whole step; this is not per-M-cycle CPU/peripheral synchronization.

use super::Machine;
use crate::cpu::CpuError;

const DMA_STALL_QUANTUM_T_CYCLES: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepOrigin {
    Cpu,
    VramDmaStall,
}

struct ScheduledStep {
    cycles: u8,
    origin: StepOrigin,
}

impl Machine {
    /// Choose which component consumes the next step before advancing time.
    ///
    /// A stalled CPU cannot fetch instructions or service interrupts, while
    /// peripheral clocks (including DMA and interrupt requests) keep running.
    /// Errors from the CPU are propagated without ticking the bus or counters.
    fn schedule_step(&mut self) -> Result<ScheduledStep, CpuError> {
        if self.bus.vram_dma_cpu_stalled() {
            Ok(ScheduledStep {
                cycles: DMA_STALL_QUANTUM_T_CYCLES,
                origin: StepOrigin::VramDmaStall,
            })
        } else {
            Ok(ScheduledStep {
                cycles: self.cpu.step(&mut self.bus)?,
                origin: StepOrigin::Cpu,
            })
        }
    }

    /// Execute a CPU step or a four-T-cycle VRAM DMA pause, then clock the bus.
    ///
    /// Preserves the existing `Machine::step` return value: the nominal
    /// duration of this step, regardless of whether the CPU made progress.
    /// CPU writes to timer/LCD/DMA registers still precede the whole tick.
    pub fn step(&mut self) -> Result<u8, CpuError> {
        let scheduled = self.schedule_step()?;
        let cycles = scheduled.cycles;

        // Keep the original peripheral ordering and instruction-boundary
        // semantics. A CPU error never enters this commit phase.
        self.bus.tick_t_cycles(u32::from(cycles));
        self.elapsed_t_cycles += u64::from(cycles);
        if scheduled.origin == StepOrigin::VramDmaStall {
            self.dma_stall_t_cycles += u64::from(cycles);
        }

        Ok(cycles)
    }
}
