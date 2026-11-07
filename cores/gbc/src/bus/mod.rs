// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Partial 16-bit address bus for the Game Boy Color.
//!
//! A partial CPU-visible bus, not a cycle-accurate hardware bus.
//! `cpu_access` maps reads/writes; `clock` advances peripherals and DMA.
//! LCD-mode gating, DMA source paths and DMG/CGB modes stay distinct.
//! CPU/DMA bus conflicts, per-access timing and most I/O remain unmodeled.

use crate::cartridge::{Cartridge, CgbSupport};
use crate::dma::{OamDma, VramDma};
use crate::interrupts::{Interrupt, InterruptController};
use crate::memory::{HighRam, WorkRam};
use crate::timer::Timer;
use crate::video::{ObjectRam, Ppu, VideoRam};

mod clock;
#[cfg(test)]
mod cartridge_tests;
mod cpu_access;
#[cfg(test)]
mod dma_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_tests;
#[cfg(test)]
mod vram_dma_tests;

/// Mode selected from the cartridge's header at this early stage.
///
/// The eventual boot sequence will own hardware-mode initialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HardwareMode {
    DmgCompatibility,
    Cgb,
}

pub struct Bus {
    cartridge: Cartridge,
    mode: HardwareMode,
    wram: WorkRam,
    hram: HighRam,
    vram: VideoRam,
    oam: ObjectRam,
    dma: OamDma,
    vram_dma: VramDma,
    ppu: Ppu,
    timer: Timer,
    interrupts: InterruptController,
}

impl Bus {
    pub fn new(cartridge: impl Into<Cartridge>) -> Self {
        let cartridge = cartridge.into();
        let mode = match cartridge.header().cgb_support {
            CgbSupport::DmgOnly => HardwareMode::DmgCompatibility,
            CgbSupport::Enhanced | CgbSupport::Exclusive => HardwareMode::Cgb,
        };
        let color = mode == HardwareMode::Cgb;
        Self {
            cartridge,
            mode,
            wram: WorkRam::new(color),
            hram: HighRam::new(),
            vram: VideoRam::new(color),
            oam: ObjectRam::new(),
            dma: OamDma::new(),
            vram_dma: VramDma::new(),
            ppu: Ppu::new(color),
            timer: Timer::new(),
            interrupts: InterruptController::new(),
        }
    }

    pub fn mode(&self) -> HardwareMode {
        self.mode
    }

    /// Completed scanline pixels, row-major RGB555, 160 × 144.
    /// Read-only; no framebuffer front/back swap or frontend API exists yet.
    pub fn framebuffer(&self) -> &[u16] {
        self.ppu.framebuffer()
    }

    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupts.request(interrupt);
    }

    pub fn highest_pending_interrupt(&self) -> Option<Interrupt> {
        self.interrupts.highest_pending()
    }

    /// Clear a requested interrupt flag; CPU IME/stack/vector handling is
    /// performed separately by the CPU.
    pub fn acknowledge_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupts.acknowledge(interrupt);
    }
}
