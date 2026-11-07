// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Cycle-accounting contracts for the instruction-boundary scheduler.

use super::Machine;
use crate::cartridge::RomOnlyCartridge;
use crate::cpu::CpuError;
use crate::interrupts::Interrupt;

fn fixture(code: &[u8]) -> Machine {
    assert!(code.len() < 0x0143);
    let mut rom = vec![0_u8; 0x8000];
    rom[..code.len()].copy_from_slice(code);
    rom[0x0143] = 0x80;
    Machine::new(RomOnlyCartridge::new(rom).unwrap())
}

fn configure_vram_dma(machine: &mut Machine, control: u8) {
    for (address, value) in [
        (0xFF51, 0xC0),
        (0xFF52, 0),
        (0xFF53, 0),
        (0xFF54, 0),
        (0xFF55, control),
    ] {
        assert!(machine.bus_mut().write(address, value));
    }
}

#[test]
fn cpu_steps_and_manual_bus_ticks_have_separate_cycle_accounting() {
    let mut machine = fixture(&[0x00, 0x06, 0x2A, 0x00]); // NOP; LD B,2A; NOP
    assert_eq!(machine.elapsed_t_cycles(), 0);
    assert_eq!(machine.cpu_t_cycles(), 0);
    assert_eq!(machine.dma_stall_t_cycles(), 0);

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.cpu().registers.b, 0x2A);
    assert_eq!(machine.elapsed_t_cycles(), 12);
    assert_eq!(machine.cpu_t_cycles(), 12);
    assert_eq!(machine.dma_stall_t_cycles(), 0);

    machine.bus_mut().tick_t_cycles(16);
    assert_eq!(machine.elapsed_t_cycles(), 12);
    assert_eq!(machine.cpu_t_cycles(), 12);
    assert_eq!(machine.dma_stall_t_cycles(), 0);

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.elapsed_t_cycles(), 16);
    assert_eq!(machine.cpu_t_cycles(), 16);
    assert_eq!(machine.cpu().registers.pc, 4);
}

#[test]
fn gdma_pauses_increment_only_stall_cycles_then_resume_cpu() {
    let mut machine = fixture(&[0x00, 0x00]);
    configure_vram_dma(&mut machine, 0); // 16-byte GDMA, 32 T-cycles

    for _ in 0..8 {
        assert_eq!(machine.step(), Ok(4));
        assert_eq!(machine.cpu().registers.pc, 0);
    }
    assert!(!machine.bus().vram_dma_cpu_stalled());
    assert_eq!(machine.elapsed_t_cycles(), 32);
    assert_eq!(machine.cpu_t_cycles(), 0);
    assert_eq!(machine.dma_stall_t_cycles(), 32);

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.cpu().registers.pc, 1);
    assert_eq!(machine.elapsed_t_cycles(), 36);
    assert_eq!(machine.cpu_t_cycles(), 4);
    assert_eq!(machine.dma_stall_t_cycles(), 32);
}

#[test]
fn cpu_instruction_that_starts_gdma_still_counts_as_cpu_step() {
    // LD A,00 ; LDH (55),A ; NOP
    let mut machine = fixture(&[0x3E, 0x00, 0xE0, 0x55, 0x00]);
    for (address, value) in [(0xFF51, 0xC0), (0xFF52, 0), (0xFF53, 0), (0xFF54, 0)] {
        assert!(machine.bus_mut().write(address, value));
    }
    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.step(), Ok(12));
    assert_eq!(machine.cpu().registers.pc, 4);
    assert_eq!(machine.cpu_t_cycles(), 20);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
    assert!(machine.bus().vram_dma_cpu_stalled());

    for _ in 0..5 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 40);
    assert_eq!(machine.cpu_t_cycles(), 20);
    assert_eq!(machine.dma_stall_t_cycles(), 20);

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.cpu().registers.pc, 5);
    assert_eq!(machine.elapsed_t_cycles(), 44);
    assert_eq!(machine.cpu_t_cycles(), 24);
    assert_eq!(machine.dma_stall_t_cycles(), 20);
}

#[test]
fn hdma_stall_is_separated_from_cpu_between_hblank_blocks() {
    let mut machine = fixture(&[0x18, 0xFE]); // JR -2, 12 cycles
    assert!(machine.bus_mut().write(0xFF40, 0x80));
    configure_vram_dma(&mut machine, 0x81); // two HBlank blocks

    for _ in 0..21 {
        assert_eq!(machine.step(), Ok(12));
    }
    assert_eq!(machine.elapsed_t_cycles(), 252);
    assert_eq!(machine.cpu_t_cycles(), 252);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
    assert!(machine.bus().vram_dma_cpu_stalled());

    for _ in 0..8 {
        assert_eq!(machine.step(), Ok(4));
        assert_eq!(machine.cpu().registers.pc, 0);
    }
    assert_eq!(machine.elapsed_t_cycles(), 284);
    assert_eq!(machine.cpu_t_cycles(), 252);
    assert_eq!(machine.dma_stall_t_cycles(), 32);
    assert!(!machine.bus().vram_dma_cpu_stalled());
    assert!(machine.bus().vram_dma_active()); // next HBlank still pending

    assert_eq!(machine.step(), Ok(12));
    assert_eq!(machine.elapsed_t_cycles(), 296);
    assert_eq!(machine.cpu_t_cycles(), 264);
    assert_eq!(machine.dma_stall_t_cycles(), 32);
}

#[test]
fn halt_idle_and_interrupt_entry_are_cpu_cycles_not_dma_stalls() {
    let mut machine = fixture(&[0xFB, 0x00, 0x76]); // EI; NOP; HALT
    machine.cpu_mut().registers.sp = 0xC100;
    assert_eq!(machine.step(), Ok(4)); // EI
    assert_eq!(machine.step(), Ok(4)); // NOP, IME enabled
    assert_eq!(machine.step(), Ok(4)); // HALT
    assert_eq!(machine.step(), Ok(4)); // HALT idle
    assert!(machine.cpu().is_halted());

    assert!(machine.bus_mut().write(0xFFFF, Interrupt::Timer.mask()));
    machine.bus_mut().request_interrupt(Interrupt::Timer);
    assert_eq!(machine.step(), Ok(20)); // enter IRQ
    assert_eq!(machine.cpu().registers.pc, Interrupt::Timer.vector());
    assert_eq!(machine.elapsed_t_cycles(), 36);
    assert_eq!(machine.cpu_t_cycles(), 36);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
}

#[test]
fn failed_cpu_step_never_commits_peripheral_ticks_or_cycle_counters() {
    let mut machine = fixture(&[0x10, 0x00]); // unsupported STOP; NOP
    assert!(machine.bus_mut().write(0xFF07, 0x05));
    machine.bus_mut().tick_t_cycles(15); // manually prime a TIMA edge

    assert_eq!(
        machine.step(),
        Err(CpuError::UnsupportedOpcode {
            address: 0,
            opcode: 0x10
        })
    );
    assert_eq!(machine.elapsed_t_cycles(), 0);
    assert_eq!(machine.cpu_t_cycles(), 0);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
    assert_eq!(machine.bus().read(0xFF05), Some(0));

    machine.cpu_mut().registers.pc = 1;
    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.elapsed_t_cycles(), 4);
    assert_eq!(machine.cpu_t_cycles(), 4);
    assert_eq!(machine.dma_stall_t_cycles(), 0);
    assert_eq!(machine.bus().read(0xFF05), Some(1));
}
