// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Instruction-boundary timer/CPU integration with synthetic ROM bytes.

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

#[test]
fn each_completed_nop_advances_timer_and_elapsed_time() {
    let mut machine = fixture(&[0; 64]);
    assert!(machine.bus_mut().write(0xFF07, 0x05));

    for _ in 0..3 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 12);
    assert_eq!(machine.bus().read(0xFF05), Some(0));

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.elapsed_t_cycles(), 16);
    assert_eq!(machine.bus().read(0xFF05), Some(1));

    for _ in 0..60 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 256);
    assert_eq!(machine.bus().read(0xFF04), Some(1));
    assert_eq!(machine.bus().read(0xFF05), Some(16));
    assert_eq!(machine.cpu().registers.pc, 64);
}

#[test]
fn different_instruction_lengths_account_for_returned_t_cycles() {
    // LD B,7A ; NOP ; JR -2
    let mut machine = fixture(&[0x06, 0x7A, 0x00, 0x18, 0xFE]);
    assert!(machine.bus_mut().write(0xFF07, 0x05));

    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.cpu().registers.b, 0x7A);
    assert_eq!(machine.elapsed_t_cycles(), 8);
    assert_eq!(machine.bus().read(0xFF05), Some(0));

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.elapsed_t_cycles(), 12);
    assert_eq!(machine.step(), Ok(12));
    assert_eq!(machine.elapsed_t_cycles(), 24);
    assert_eq!(machine.bus().read(0xFF05), Some(1));
    assert_eq!(machine.cpu().registers.pc, 3);
}

#[test]
fn cb_instructions_use_their_nominal_cycle_count() {
    let mut machine = fixture(&[0xCB, 0x00, 0x00]); // RLC B; NOP
    machine.cpu_mut().registers.b = 0x80;

    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.elapsed_t_cycles(), 8);
    assert_eq!(machine.cpu().registers.b, 1);
    assert_eq!(machine.cpu().registers.pc, 2);

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.elapsed_t_cycles(), 12);
    assert_eq!(machine.cpu().registers.pc, 3);
}

#[test]
fn halted_cpu_keeps_timer_running_and_wakes_with_ime_disabled() {
    let mut machine = fixture(&[0x76, 0x00]); // HALT; NOP
    assert!(machine.bus_mut().write(0xFFFF, Interrupt::Timer.mask()));
    assert!(machine.bus_mut().write(0xFF05, 0xFF));
    assert!(machine.bus_mut().write(0xFF06, 0x42));
    assert!(machine.bus_mut().write(0xFF07, 0x05));

    assert_eq!(machine.step(), Ok(4)); // HALT
    assert!(machine.cpu().is_halted());

    for _ in 0..3 {
        assert_eq!(machine.step(), Ok(4)); // reach 16 T-cycles
        assert!(machine.cpu().is_halted());
    }
    assert_eq!(machine.elapsed_t_cycles(), 16);
    assert_eq!(machine.bus().read(0xFF05), Some(0));
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));

    assert_eq!(machine.step(), Ok(4)); // delayed TIMA reload
    assert_eq!(machine.elapsed_t_cycles(), 20);
    assert_eq!(machine.bus().read(0xFF05), Some(0x42));
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE4));

    assert_eq!(machine.step(), Ok(4)); // wake, execute NOP without servicing
    assert!(!machine.cpu().is_halted());
    assert_eq!(machine.cpu().registers.pc, 2);
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE4));
    assert_eq!(machine.elapsed_t_cycles(), 24);
}

#[test]
fn timer_interrupt_services_halted_cpu_on_the_following_step() {
    // EI; NOP; HALT; NOP
    let mut machine = fixture(&[0xFB, 0x00, 0x76, 0x00]);
    machine.cpu_mut().registers.sp = 0xC100;
    assert!(machine.bus_mut().write(0xFFFF, Interrupt::Timer.mask()));
    assert!(machine.bus_mut().write(0xFF05, 0xFF));
    assert!(machine.bus_mut().write(0xFF06, 0x35));
    assert!(machine.bus_mut().write(0xFF07, 0x05));

    assert_eq!(machine.step(), Ok(4)); // EI
    assert_eq!(machine.step(), Ok(4)); // NOP: IME becomes enabled
    assert!(machine.cpu().interrupts_enabled());
    assert_eq!(machine.step(), Ok(4)); // HALT
    assert!(machine.cpu().is_halted());
    assert_eq!(machine.step(), Ok(4)); // timer overflows at T=16
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));
    assert_eq!(machine.step(), Ok(4)); // timer reload requests IRQ at T=20
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE4));
    assert_eq!(machine.elapsed_t_cycles(), 20);

    assert_eq!(machine.step(), Ok(20)); // IRQ enters before next opcode
    assert_eq!(machine.elapsed_t_cycles(), 40);
    assert!(!machine.cpu().is_halted());
    assert!(!machine.cpu().interrupts_enabled());
    assert_eq!(machine.cpu().registers.pc, Interrupt::Timer.vector());
    assert_eq!(machine.cpu().registers.sp, 0xC0FE);
    assert_eq!(machine.bus().read(0xC0FE), Some(3));
    assert_eq!(machine.bus().read(0xC0FF), Some(0));
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));
}

#[test]
fn timer_irq_generated_by_a_step_is_seen_on_the_next_step_only() {
    let mut machine = fixture(&[0xFB, 0x00, 0x00, 0x00, 0x00]);
    machine.cpu_mut().registers.sp = 0xD000;
    assert!(machine.bus_mut().write(0xFFFF, Interrupt::Timer.mask()));
    assert!(machine.bus_mut().write(0xFF05, 0xFF));
    assert!(machine.bus_mut().write(0xFF07, 0x05));

    for _ in 0..4 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 16);
    assert_eq!(machine.cpu().registers.pc, 4);
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));

    assert_eq!(machine.step(), Ok(4));
    assert_eq!(machine.cpu().registers.pc, 5);
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE4));

    assert_eq!(machine.step(), Ok(20));
    assert_eq!(machine.elapsed_t_cycles(), 40);
    assert_eq!(machine.cpu().registers.pc, Interrupt::Timer.vector());
    assert_eq!(machine.bus().read(0xCFFE), Some(5));
    assert_eq!(machine.bus().read(0xCFFF), Some(0));
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));
}

#[test]
fn an_unsupported_instruction_does_not_advance_timer_or_cycle_counter() {
    let mut machine = fixture(&[0x10]); // STOP remains unsupported
    assert!(machine.bus_mut().write(0xFF07, 0x05));
    // Put divider one T-cycle before the next TIMA falling edge.
    machine.bus_mut().tick_t_cycles(15);
    assert_eq!(machine.bus().read(0xFF05), Some(0));

    assert_eq!(
        machine.step(),
        Err(CpuError::UnsupportedOpcode {
            address: 0,
            opcode: 0x10
        })
    );
    assert_eq!(machine.cpu().registers.pc, 0);
    assert_eq!(machine.elapsed_t_cycles(), 0);
    assert_eq!(machine.bus().read(0xFF05), Some(0));

    // Manual external ticking is allowed but not included in Machine's count.
    machine.bus_mut().tick_t_cycles(1);
    assert_eq!(machine.bus().read(0xFF05), Some(1));
    assert_eq!(machine.elapsed_t_cycles(), 0);
}

#[test]
fn machine_cpu_can_enable_lcd_and_advance_ppu_modes() {
    // LD A,80 ; LDH (40),A ; followed by NOP instructions.
    let mut machine = fixture(&[0x3E, 0x80, 0xE0, 0x40]);
    assert_eq!(machine.bus().read(0xFF40), Some(0));

    assert_eq!(machine.step(), Ok(8));
    assert_eq!(machine.bus().read(0xFF44), Some(0));
    assert_eq!(machine.step(), Ok(12));
    assert_eq!(machine.bus().read(0xFF40), Some(0x80));
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 2);

    // LCDC writes happen before the runner clocks the 12-cycle LDH.
    for _ in 0..17 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 3); // 80 dots
    for _ in 0..43 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 0); // 252 dots
    for _ in 0..51 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().read(0xFF44), Some(1)); // 456 dots
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 2);
    assert_eq!(machine.elapsed_t_cycles(), 8 + 12 + 111 * 4);
}

#[test]
fn lcd_keeps_advancing_during_halt_idle_steps() {
    let mut machine = fixture(&[0x76]); // HALT
    assert!(machine.bus_mut().write(0xFF40, 0x80));

    assert_eq!(machine.step(), Ok(4));
    assert!(machine.cpu().is_halted());
    for _ in 0..19 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 3);
    for _ in 0..94 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().read(0xFF44), Some(1));
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 2);
    assert_eq!(machine.elapsed_t_cycles(), 456);
}

#[test]
fn ppu_stat_irq_is_serviced_at_the_next_machine_step() {
    // EI ; NOP ; HALT. Mode 0 STAT IRQ is generated at 252 LCD dots.
    let mut machine = fixture(&[0xFB, 0x00, 0x76]);
    machine.cpu_mut().registers.sp = 0xC100;
    assert!(machine.bus_mut().write(0xFFFF, Interrupt::LcdStat.mask()));
    assert!(machine.bus_mut().write(0xFF41, 0x08));
    assert!(machine.bus_mut().write(0xFF40, 0x80));

    assert_eq!(machine.step(), Ok(4)); // EI
    assert_eq!(machine.step(), Ok(4)); // NOP, enables IME
    assert_eq!(machine.step(), Ok(4)); // HALT
    for _ in 0..60 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 252);
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 0);
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE2));

    assert_eq!(machine.step(), Ok(20));
    assert_eq!(machine.cpu().registers.pc, Interrupt::LcdStat.vector());
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));
    assert_eq!(machine.bus().read(0xC0FE), Some(3));
    assert_eq!(machine.bus().read(0xC0FF), Some(0));
}

#[test]
fn ppu_requests_vblank_once_at_line_144_during_cpu_halt() {
    let mut machine = fixture(&[0xFB, 0x00, 0x76]);
    machine.cpu_mut().registers.sp = 0xC100;
    assert!(machine.bus_mut().write(0xFFFF, Interrupt::VBlank.mask()));
    assert!(machine.bus_mut().write(0xFF40, 0x80));

    assert_eq!(machine.step(), Ok(4)); // EI
    assert_eq!(machine.step(), Ok(4)); // NOP
    assert_eq!(machine.step(), Ok(4)); // HALT
    assert!(machine.cpu().is_halted());
    // 144 * 456 = 65,664 dots; the first three CPU steps took 12.
    for _ in 0..16_413 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.elapsed_t_cycles(), 65_664);
    assert_eq!(machine.bus().read(0xFF44), Some(144));
    assert_eq!(machine.bus().read(0xFF41).unwrap() & 3, 1);
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE1));

    assert_eq!(machine.step(), Ok(20));
    assert_eq!(machine.cpu().registers.pc, Interrupt::VBlank.vector());
    assert_eq!(machine.bus().read(0xFF0F), Some(0xE0));
    assert_eq!(machine.elapsed_t_cycles(), 65_684);
}

#[test]
fn machine_exposes_rgb555_background_pixels_after_cpu_halt_steps() {
    // The fixture runs in native CGB mode (header flag 0x80).
    let mut machine = fixture(&[0x76]); // HALT
    assert_eq!(
        machine.bus().framebuffer().len(),
        crate::SCREEN_WIDTH * crate::SCREEN_HEIGHT
    );

    assert!(machine.bus_mut().write(0x8000, 0xFF)); // tile 0 row 0, color 1
    assert!(machine.bus_mut().write(0x9801, 1)); // next map cell uses blank tile 1
    assert!(machine.bus_mut().write(0xFF68, 0x82)); // palette 0, color 1
    assert!(machine.bus_mut().write(0xFF69, 0xE0)); // RGB555 green, low byte
    assert!(machine.bus_mut().write(0xFF69, 0x03)); // high byte
    assert!(machine.bus_mut().write(0xFF40, 0x91)); // LCD + BG + 8000 tile data

    for _ in 0..62 {
        assert_eq!(machine.step(), Ok(4));
    }
    assert_eq!(machine.bus().framebuffer()[0], 0x7FFF); // not rendered yet
    assert_eq!(machine.step(), Ok(4)); // nominal transfer ends at dot 252
    assert_eq!(machine.bus().framebuffer()[0], 0x03E0);
    assert_eq!(machine.bus().framebuffer()[7], 0x03E0);
    assert_eq!(machine.bus().framebuffer()[8], 0x7FFF);
    assert_eq!(machine.bus().framebuffer()[crate::SCREEN_WIDTH], 0x7FFF);
    assert_eq!(machine.elapsed_t_cycles(), 252);
}
