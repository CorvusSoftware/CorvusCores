// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! MBC3 real-time-clock counters and snapshot latch.
//!
//! Live counters advance only when the host explicitly supplies elapsed
//! seconds. CPU T-cycles and the host wall clock are deliberately NOT used:
//! the RTC is an independent oscillator, including while the CPU is halted.
//! Host-time synchronization, battery persistence and out-of-range register
//! write quirks belong to later integration work.

const SECONDS_PER_DAY: u64 = 86_400;
const DAYS_PER_WRAP: u64 = 512;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RtcRegisters {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
    halted: bool,
    carry: bool,
}

impl RtcRegisters {
    fn read(self, register: u8) -> u8 {
        match register {
            0x08 => self.seconds,
            0x09 => self.minutes,
            0x0A => self.hours,
            0x0B => self.days as u8,
            0x0C => u8::from(self.days & 0x100 != 0)
                | if self.halted { 0x40 } else { 0 }
                | if self.carry { 0x80 } else { 0 },
            _ => 0xFF,
        }
    }

    fn write(&mut self, register: u8, value: u8) {
        match register {
            0x08 => self.seconds = value & 0x3F,
            0x09 => self.minutes = value & 0x3F,
            0x0A => self.hours = value & 0x1F,
            0x0B => self.days = (self.days & 0x100) | u16::from(value),
            0x0C => {
                self.days = (self.days & 0xFF) | (u16::from(value & 1) << 8);
                self.halted = value & 0x40 != 0;
                self.carry = value & 0x80 != 0;
            }
            _ => {}
        }
    }

    fn advance_seconds(&mut self, seconds: u64) {
        if self.halted || seconds == 0 {
            return;
        }

        // Split the delta before adding to avoid overflow for u64::MAX.
        // Clock registers are normally written with valid 00–59 / 00–23
        // values. Out-of-range writes are bit-masked, but hardware rollover
        // for such values has not been established.
        let seconds_in_day = u64::from(self.hours) * 3_600
            + u64::from(self.minutes) * 60
            + u64::from(self.seconds)
            + seconds % SECONDS_PER_DAY;
        let elapsed_days = seconds / SECONDS_PER_DAY + seconds_in_day / SECONDS_PER_DAY;
        let seconds_in_day = seconds_in_day % SECONDS_PER_DAY;

        self.hours = (seconds_in_day / 3_600) as u8;
        self.minutes = ((seconds_in_day % 3_600) / 60) as u8;
        self.seconds = (seconds_in_day % 60) as u8;

        let accumulated_days = u64::from(self.days) + elapsed_days;
        if accumulated_days >= DAYS_PER_WRAP {
            self.carry = true;
        }
        self.days = (accumulated_days % DAYS_PER_WRAP) as u16;
    }
}

#[derive(Debug, Default)]
pub(super) struct Rtc {
    live: RtcRegisters,
    latched: RtcRegisters,
    latch_armed: bool,
}

impl Rtc {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Reads use the most recently latched snapshot, never the live clock.
    pub(super) fn read(&self, register: u8) -> u8 {
        self.latched.read(register)
    }

    /// Writes target the live counter, not an existing frozen snapshot.
    pub(super) fn write(&mut self, register: u8, value: u8) {
        self.live.write(register, value);
    }

    /// Only an exact 00-to-01 command sequence captures a new snapshot.
    pub(super) fn write_latch(&mut self, value: u8) {
        match value {
            0 => self.latch_armed = true,
            1 if self.latch_armed => {
                self.latched = self.live;
                self.latch_armed = false;
            }
            _ => self.latch_armed = false,
        }
    }

    /// Advance only the live clock; the latched values stay frozen.
    pub(super) fn advance_seconds(&mut self, seconds: u64) {
        self.live.advance_seconds(seconds);
    }
}
