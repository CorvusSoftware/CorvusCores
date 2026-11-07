// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Normal-speed peripheral clock and DMA source paths.
//!
//! The ordering within one T-cycle is intentional: timer, OAM DMA,
//! CGB VRAM DMA, then LCD PPU and HBlank edge detection.
//! CPU accesses still happen at instruction boundaries, not each T-cycle.

use super::Bus;

impl Bus {
    /// Advance timer, OAM DMA, CGB VRAM DMA and PPU in normal-speed T-cycles.
    /// CPU executes at instruction boundaries rather than at individual bus
    /// accesses, and CGB double-speed clock conversion is not modeled.
    pub fn tick_t_cycles(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.timer.tick(1, &mut self.interrupts);
            if let Some((source, destination)) = self.dma.tick_t_cycle() {
                let value = self.read_dma_source(source);
                self.oam.write(destination, value);
            }
            if let Some((source, destination)) = self.vram_dma.tick_t_cycle() {
                let value = self.read_vram_dma_source(source);
                // The LCD engine and VRAM DMA have independent access paths.
                // Bank selection follows VBK at the moment of the transfer.
                self.vram.write(destination, value);
            }
            let was_hblank = self.ppu.visible_hblank();
            self.ppu
                .tick(1, &self.vram, &self.oam, &mut self.interrupts);
            if !was_hblank && self.ppu.visible_hblank() {
                self.vram_dma.begin_hblank();
            }
        }
    }

    /// True if GDMA or an active HBlank block currently holds the CPU.
    pub fn vram_dma_cpu_stalled(&self) -> bool {
        self.vram_dma.stalls_cpu()
    }

    /// True for ongoing GDMA or a HDMA waiting for the next visible HBlank.
    pub fn vram_dma_active(&self) -> bool {
        self.vram_dma.active()
    }

    /// True while FF46 has an unfinished OAM transfer.
    pub fn oam_dma_active(&self) -> bool {
        self.dma.active()
    }

    /// VRAM DMA source reads do not consult CPU LCD-mode access gating.
    /// Unsupported, absent cartridge RAM and invalid VRAM sources are FF
    /// placeholders, not claims about physical open-bus signal behavior.
    fn read_vram_dma_source(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.cartridge.read_rom(address).unwrap_or(0xFF),
            0xA000..=0xBFFF => 0xFF,
            0xC000..=0xDFFF => self.wram.read(address).unwrap_or(0xFF),
            _ => 0xFF,
        }
    }

    /// Source reads bypass CPU LCD-mode access restrictions.
    /// Pages outside 00–DF are not currently hardware-accurate: an
    /// unsupported source gives the deterministic placeholder FF.
    fn read_dma_source(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.cartridge.read_rom(address).unwrap_or(0xFF),
            0x8000..=0x9FFF => self.vram.read(address).unwrap_or(0xFF),
            0xA000..=0xBFFF => 0xFF, // ROM-only cartridge RAM is absent.
            0xC000..=0xDFFF => self.wram.read(address).unwrap_or(0xFF),
            _ => 0xFF,
        }
    }
}
