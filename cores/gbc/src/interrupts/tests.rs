// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Interrupt, InterruptController};

#[test]
fn requests_are_retained_without_enable_or_cpu_service() {
    let mut interrupts = InterruptController::new();
    assert_eq!(interrupts.read_if(), 0xE0);
    assert_eq!(interrupts.read_ie(), 0);
    interrupts.request(Interrupt::Timer);
    assert_eq!(interrupts.read_if(), 0xE4);
    assert_eq!(interrupts.pending_mask(), 0);
    assert_eq!(interrupts.highest_pending(), None);

    interrupts.write_ie(0x04);
    assert_eq!(interrupts.pending_mask(), 0x04);
    assert_eq!(interrupts.highest_pending(), Some(Interrupt::Timer));
    interrupts.acknowledge(Interrupt::Timer);
    assert_eq!(interrupts.read_if(), 0xE0);
    assert_eq!(interrupts.read_ie(), 0x04);
}

#[test]
fn priority_and_vectors_match_all_five_interrupts() {
    let cases = [
        (Interrupt::VBlank, 0x01, 0x0040),
        (Interrupt::LcdStat, 0x02, 0x0048),
        (Interrupt::Timer, 0x04, 0x0050),
        (Interrupt::Serial, 0x08, 0x0058),
        (Interrupt::Joypad, 0x10, 0x0060),
    ];
    let mut interrupts = InterruptController::new();
    interrupts.write_ie(0x1F);
    for &(interrupt, mask, vector) in &cases {
        assert_eq!(interrupt.mask(), mask);
        assert_eq!(interrupt.vector(), vector);
        interrupts.request(interrupt);
    }
    for &(interrupt, _, _) in &cases {
        assert_eq!(interrupts.highest_pending(), Some(interrupt));
        interrupts.acknowledge(interrupt);
    }
    assert_eq!(interrupts.pending_mask(), 0);
}

#[test]
fn ie_high_bits_do_not_create_interrupts() {
    let mut interrupts = InterruptController::new();
    interrupts.write_ie(0xE0);
    interrupts.write_if(0xFF);
    assert_eq!(interrupts.read_if(), 0xFF);
    assert_eq!(interrupts.read_ie(), 0xE0);
    assert_eq!(interrupts.pending_mask(), 0);
    interrupts.write_ie(0xE4);
    assert_eq!(interrupts.highest_pending(), Some(Interrupt::Timer));
}

#[test]
fn software_writes_can_request_and_clear_individual_flags() {
    let mut interrupts = InterruptController::new();
    interrupts.write_if(0x1B);
    assert_eq!(interrupts.read_if(), 0xFB);
    interrupts.request(Interrupt::Timer);
    assert_eq!(interrupts.read_if(), 0xFF);
    interrupts.write_if(Interrupt::Serial.mask());
    assert_eq!(interrupts.read_if(), 0xE8);
    interrupts.acknowledge(Interrupt::Serial);
    assert_eq!(interrupts.read_if(), 0xE0);
}
