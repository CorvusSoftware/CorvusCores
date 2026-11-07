// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Partial 16-bit address bus for the Game Boy Color.
//!
//! This is an untimed memory-routing foundation, NOT the final CPU-visible bus.
//! Unimplemented I/O, precisely timed CPU interrupt service, boot ROM
//! mapping, DMA and PPU access restrictions are deliberately not faked. An absent read returns `None`,
//! and an unsupported write returns `false`.

use crate::cartridge::{CgbSupport, RomOnlyCartridge};
use crate::interrupts::{Interrupt, InterruptController};
use crate::memory::{HighRam, WorkRam};
use crate::timer::Timer;
use crate::video::{ObjectRam, VideoRam};

#[cfg(test)]
mod timing_tests;

/// Mode selected from the cartridge's header at this early stage.
///
/// The eventual boot sequence will own hardware-mode initialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HardwareMode {
    DmgCompatibility,
    Cgb,
}

pub struct Bus {
    cartridge: RomOnlyCartridge,
    mode: HardwareMode,
    wram: WorkRam,
    hram: HighRam,
    vram: VideoRam,
    oam: ObjectRam,
    timer: Timer,
    interrupts: InterruptController,
}

impl Bus {
    pub fn new(cartridge: RomOnlyCartridge) -> Self {
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
            timer: Timer::new(),
            interrupts: InterruptController::new(),
        }
    }

    pub fn mode(&self) -> HardwareMode {
        self.mode
    }

    /// Advance the divider and timers by CPU T-cycles. The current Cpu::step
    /// returns nominal cycles but does not call this method automatically.
    pub fn tick_t_cycles(&mut self, cycles: u32) {
        self.timer.tick(cycles, &mut self.interrupts);
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

    /// Read an implemented address. `None` means not yet modeled, not
    /// necessarily a specific value returned by real hardware.
    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            0x0000..=0x7FFF => self.cartridge.read_rom(address),
            0x8000..=0x9FFF => self.vram.read(address),
            // ROM-only cartridges have no external RAM installed.
            0xA000..=0xBFFF => Some(0xFF),
            0xC000..=0xFDFF => self.wram.read(address),
            0xFE00..=0xFE9F => self.oam.read(address),
            0xFF04..=0xFF07 => self.timer.read(address),
            0xFF0F => Some(self.interrupts.read_if()),
            0xFF4F => self.vram.read_bank_register(),
            0xFF70 => self.wram.read_bank_register(),
            0xFF80..=0xFFFE => self.hram.read(address),
            0xFFFF => Some(self.interrupts.read_ie()),
            _ => None,
        }
    }

    /// Write to an implemented address. `false` means the target is
    /// unsupported; writes to ROM or absent cartridge RAM are valid no-ops.
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => true,
            0x8000..=0x9FFF => self.vram.write(address, value),
            0xC000..=0xFDFF => self.wram.write(address, value),
            0xFE00..=0xFE9F => self.oam.write(address, value),
            0xFF04..=0xFF07 => self.timer.write(address, value),
            0xFF0F => {
                self.interrupts.write_if(value);
                true
            }
            0xFF4F => self.vram.write_bank_register(value),
            0xFF70 => self.wram.write_bank_register(value),
            0xFF80..=0xFFFE => self.hram.write(address, value),
            0xFFFF => {
                self.interrupts.write_ie(value);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bus, HardwareMode};
    use crate::cartridge::RomOnlyCartridge;

    fn test_bus(flag: u8) -> Bus {
        let mut rom = vec![0_u8; 0x8000];
        rom[0] = 0x42;
        rom[0x0143] = flag;
        Bus::new(RomOnlyCartridge::new(rom).unwrap())
    }

    #[test]
    fn routes_rom_without_mutating_rom_only_cartridges() {
        let mut bus = test_bus(0x80);
        assert_eq!(bus.read(0), Some(0x42));
        assert!(bus.write(0, 0x55));
        assert_eq!(bus.read(0), Some(0x42));
        assert_eq!(bus.read(0xA000), Some(0xFF));
        assert!(bus.write(0xA000, 0x33));
        assert_eq!(bus.read(0xA000), Some(0xFF));
    }

    #[test]
    fn routes_vram_wram_echo_oam_and_hram() {
        let mut bus = test_bus(0xC0);
        for (addr, value) in [
            (0x8000, 1),
            (0x9FFF, 2),
            (0xC000, 3),
            (0xD000, 4),
            (0xFE00, 5),
            (0xFE9F, 6),
            (0xFF80, 7),
            (0xFFFE, 8),
        ] {
            assert!(bus.write(addr, value));
            assert_eq!(bus.read(addr), Some(value));
        }
        assert_eq!(bus.read(0xE000), Some(3));
        assert_eq!(bus.read(0xF000), Some(4));
        assert!(bus.write(0xFDFF, 9));
        assert_eq!(bus.read(0xDDFF), Some(9));
        assert_eq!(bus.read(0xFEA0), None);
        assert!(!bus.write(0xFEA0, 1));
    }

    #[test]
    fn banking_registers_route_to_their_own_components() {
        let mut bus = test_bus(0x80);
        assert_eq!(bus.mode(), HardwareMode::Cgb);
        assert!(bus.write(0x8000, 0x10));
        assert!(bus.write(0xFF4F, 0x01));
        assert_eq!(bus.read(0xFF4F), Some(0xFF));
        assert!(bus.write(0x8000, 0x20));
        assert!(bus.write(0xFF4F, 0));
        assert_eq!(bus.read(0x8000), Some(0x10));
        assert!(bus.write(0xFF4F, 1));
        assert_eq!(bus.read(0x8000), Some(0x20));

        assert!(bus.write(0xD000, 0x40));
        assert!(bus.write(0xFF70, 3));
        assert!(bus.write(0xD000, 0x60));
        assert_eq!(bus.read(0xFF70), Some(0xFB));
        assert!(bus.write(0xFF70, 0));
        assert_eq!(bus.read(0xD000), Some(0x40));
        assert_eq!(bus.read(0xFF70), Some(0xF8));
        assert!(bus.write(0xFF70, 3));
        assert_eq!(bus.read(0xD000), Some(0x60));
    }

    #[test]
    fn original_game_boy_titles_use_compatibility_banking() {
        let mut bus = test_bus(0x00);
        assert_eq!(bus.mode(), HardwareMode::DmgCompatibility);
        assert!(!bus.write(0xFF4F, 1));
        assert!(!bus.write(0xFF70, 3));
        assert_eq!(bus.read(0xFF4F), None);
        assert_eq!(bus.read(0xFF70), None);
        assert!(bus.write(0xD000, 0x77));
        assert_eq!(bus.read(0xD000), Some(0x77));
    }

    #[test]
    fn unsupported_registers_are_not_silently_fake_mapped() {
        let mut bus = test_bus(0x80);
        for addr in [0xFEA0, 0xFF00, 0xFF46] {
            assert_eq!(bus.read(addr), None);
            assert!(!bus.write(addr, 0xAA));
        }
    }
}
