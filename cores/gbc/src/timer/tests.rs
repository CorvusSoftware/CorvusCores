// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

use super::Timer;
use crate::interrupts::{Interrupt, InterruptController};

#[test]
fn divider_counts_even_when_tima_is_disabled_and_wraps() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert_eq!(timer.read(0xFF04), Some(0));
    timer.tick(255, &mut interrupts);
    assert_eq!(timer.read(0xFF04), Some(0));
    timer.tick(1, &mut interrupts);
    assert_eq!(timer.read(0xFF04), Some(1));
    assert_eq!(timer.read(0xFF05), Some(0));
    timer.tick(65_280, &mut interrupts);
    assert_eq!(timer.read(0xFF04), Some(0));
    assert_eq!(interrupts.read_if(), 0xE0);
}

#[test]
fn all_four_timer_clock_sources_have_correct_falling_edge_periods() {
    for (frequency, period) in [(0, 1024), (1, 16), (2, 64), (3, 256)] {
        let mut timer = Timer::new();
        let mut interrupts = InterruptController::new();
        assert!(timer.write(0xFF07, 0x04 | frequency));
        timer.tick(period - 1, &mut interrupts);
        assert_eq!(timer.read(0xFF05), Some(0), "TAC frequency {frequency}");
        timer.tick(1, &mut interrupts);
        assert_eq!(timer.read(0xFF05), Some(1), "TAC frequency {frequency}");
        timer.tick(period, &mut interrupts);
        assert_eq!(timer.read(0xFF05), Some(2), "TAC frequency {frequency}");
    }
}

#[test]
fn div_write_resets_counter_and_can_trigger_a_falling_edge() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert!(timer.write(0xFF07, 0x05));
    timer.tick(8, &mut interrupts); // divider bit 3 is high
    assert_eq!(timer.read(0xFF05), Some(0));
    assert!(timer.write(0xFF04, 0xAA));
    assert_eq!(timer.read(0xFF04), Some(0));
    assert_eq!(timer.read(0xFF05), Some(1));
    timer.tick(16, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(2));
}

#[test]
fn switching_active_tac_sources_can_tick_early() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert!(timer.write(0xFF07, 0x05));
    timer.tick(8, &mut interrupts);
    assert!(timer.write(0xFF07, 0x04)); // bit 3 high -> bit 9 low
    assert_eq!(timer.read(0xFF05), Some(1));
    assert!(timer.write(0xFF07, 0x00)); // disable not modeled as CGB pulse
    assert_eq!(timer.read(0xFF05), Some(1));
    timer.tick(2048, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(1));
}

#[test]
fn overflow_reloads_after_four_t_cycles_and_requests_interrupt() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert!(timer.write(0xFF06, 0xA7));
    assert!(timer.write(0xFF05, 0xFF));
    assert!(timer.write(0xFF07, 0x05));
    timer.tick(16, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(0));
    assert_eq!(interrupts.read_if(), 0xE0);
    timer.tick(3, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(0));
    assert_eq!(interrupts.read_if(), 0xE0);
    timer.tick(1, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(0xA7));
    assert_eq!(interrupts.read_if(), 0xE4);
    assert_eq!(interrupts.highest_pending(), None);
    interrupts.write_ie(Interrupt::Timer.mask());
    assert_eq!(interrupts.highest_pending(), Some(Interrupt::Timer));
}

#[test]
fn writing_tima_during_pending_reload_cancels_the_interrupt() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert!(timer.write(0xFF05, 0xFF));
    assert!(timer.write(0xFF07, 0x05));
    timer.tick(16, &mut interrupts);
    assert!(timer.write(0xFF05, 0x55));
    assert!(timer.write(0xFF07, 0x00));
    timer.tick(8, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(0x55));
    assert_eq!(interrupts.read_if(), 0xE0);
}

#[test]
fn writing_modulo_before_reload_affects_reloaded_value() {
    let mut timer = Timer::new();
    let mut interrupts = InterruptController::new();
    assert!(timer.write(0xFF05, 0xFF));
    assert!(timer.write(0xFF06, 0x12));
    assert!(timer.write(0xFF07, 0x05));
    timer.tick(16, &mut interrupts);
    assert!(timer.write(0xFF06, 0xB3));
    timer.tick(4, &mut interrupts);
    assert_eq!(timer.read(0xFF05), Some(0xB3));
    assert_eq!(interrupts.read_if(), 0xE4);
}

#[test]
fn tac_unused_bits_read_high_and_only_3_control_bits_are_stored() {
    let mut timer = Timer::new();
    assert_eq!(timer.read(0xFF07), Some(0xF8));
    assert!(timer.write(0xFF07, 0xFF));
    assert_eq!(timer.read(0xFF07), Some(0xFF));
    assert!(timer.write(0xFF07, 0x80));
    assert_eq!(timer.read(0xFF07), Some(0xF8));
    assert_eq!(timer.read(0xFF03), None);
    assert!(!timer.write(0xFF08, 0x77));
}
