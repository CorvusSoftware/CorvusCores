// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic MBC3 RTC snapshots, calendar rollover and halt/carry tests.

use super::rtc::Rtc;

fn latch(rtc: &mut Rtc) {
    rtc.write_latch(0);
    rtc.write_latch(1);
}

#[test]
fn rtc_starts_zero_and_latched_snapshot_stays_frozen() {
    let mut rtc = Rtc::new();
    for register in 0x08..=0x0C {
        assert_eq!(rtc.read(register), 0);
    }
    rtc.advance_seconds(65);
    assert_eq!(rtc.read(0x08), 0); // not latched yet
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 5);
    assert_eq!(rtc.read(0x09), 1);
    rtc.advance_seconds(3600);
    assert_eq!(rtc.read(0x0A), 0);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0A), 1);
    assert_eq!(rtc.read(0x08), 5);
}

#[test]
fn seconds_minutes_hours_roll_into_next_day() {
    let mut rtc = Rtc::new();
    rtc.write(0x08, 59);
    rtc.write(0x09, 59);
    rtc.write(0x0A, 23);
    rtc.write(0x0B, 8);
    rtc.advance_seconds(1);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 0);
    assert_eq!(rtc.read(0x09), 0);
    assert_eq!(rtc.read(0x0A), 0);
    assert_eq!(rtc.read(0x0B), 9);
    assert_eq!(rtc.read(0x0C), 0);
}

#[test]
fn day_511_wraps_to_zero_and_carry_sticks_until_cleared() {
    let mut rtc = Rtc::new();
    rtc.write(0x0B, 0xFF);
    rtc.write(0x0C, 0x01); // day bit 8, running
    rtc.advance_seconds(86_400);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0B), 0);
    assert_eq!(rtc.read(0x0C), 0x80); // overflow
    rtc.advance_seconds(3 * 86_400);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0B), 3);
    assert_eq!(rtc.read(0x0C), 0x80); // sticky

    rtc.write(0x0C, 0); // software clears carry
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0B), 3);
    assert_eq!(rtc.read(0x0C), 0);
}

#[test]
fn halted_clock_does_not_advance_and_resumes_without_catching_up() {
    let mut rtc = Rtc::new();
    rtc.write(0x08, 17);
    rtc.write(0x0C, 0x40); // halt
    rtc.advance_seconds(123_456_789);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 17);
    assert_eq!(rtc.read(0x0C), 0x40);
    rtc.write(0x0C, 0);
    rtc.advance_seconds(2);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 19);
    assert_eq!(rtc.read(0x0C), 0);
}

#[test]
fn day_high_halt_carry_and_low_day_writes_are_independent() {
    let mut rtc = Rtc::new();
    rtc.write(0x0B, 0xEF);
    rtc.write(0x0C, 0xC1); // day high, halt, carry
    rtc.write(0x0B, 0x12); // preserve day bit 8
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0B), 0x12);
    assert_eq!(rtc.read(0x0C), 0xC1);

    rtc.write(0x0C, 0x40); // clear carry and day high
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0B), 0x12);
    assert_eq!(rtc.read(0x0C), 0x40);
    rtc.write(0x0C, 0xFF); // reserved bits 1–5 ignored
    latch(&mut rtc);
    assert_eq!(rtc.read(0x0C), 0xC1);
}

#[test]
fn only_zero_followed_by_one_refreshes_latched_clock() {
    let mut rtc = Rtc::new();
    rtc.advance_seconds(1);
    rtc.write_latch(1);
    assert_eq!(rtc.read(0x08), 0);
    rtc.write_latch(0);
    rtc.write_latch(2); // breaks the sequence
    rtc.write_latch(1);
    assert_eq!(rtc.read(0x08), 0);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 1);
    rtc.advance_seconds(1);
    rtc.write_latch(1);
    assert_eq!(rtc.read(0x08), 1);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 2);
}

#[test]
fn rtc_register_masks_ignore_reserved_bits_on_write() {
    let mut rtc = Rtc::new();
    rtc.write(0x08, 0xFF);
    rtc.write(0x09, 0xFF);
    rtc.write(0x0A, 0xFF);
    rtc.write(0x0C, 0xFE);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), 0x3F);
    assert_eq!(rtc.read(0x09), 0x3F);
    assert_eq!(rtc.read(0x0A), 0x1F);
    assert_eq!(rtc.read(0x0C), 0xC0);
    assert_eq!(rtc.read(0x07), 0xFF);
}

#[test]
fn huge_elapsed_seconds_wrap_without_overflow_or_per_second_loops() {
    let mut rtc = Rtc::new();
    rtc.advance_seconds(u64::MAX);
    latch(&mut rtc);
    let fraction = u64::MAX % 86_400;
    let days = (u64::MAX / 86_400) % 512;
    assert_eq!(rtc.read(0x08), (fraction % 60) as u8);
    assert_eq!(rtc.read(0x09), ((fraction / 60) % 60) as u8);
    assert_eq!(rtc.read(0x0A), (fraction / 3600) as u8);
    assert_eq!(rtc.read(0x0B), days as u8);
    assert_eq!(rtc.read(0x0C), 0x80 | ((days >> 8) as u8));

    rtc.advance_seconds(0);
    latch(&mut rtc);
    assert_eq!(rtc.read(0x08), (fraction % 60) as u8);
}
