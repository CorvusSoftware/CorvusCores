// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Channel 2: Square wave (no sweep).
//!
//! Registers:
//! - NR21: Sound length, wave pattern duty (FF16)
//! - NR22: Volume envelope (FF17)
//! - NR23: Frequency low (FF18)
//! - NR24: Frequency high, trigger, length enable (FF19)

use super::CPU_CLOCK;

/// Square wave duty patterns (2-bit index).
const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

/// Frequency counter divisor for channel 2.
const CHANNEL2_FREQUENCY_DIVISOR: u32 = 1;

/// Length counter clock frequency (256 Hz).
const LENGTH_CLOCK: u32 = 256;

/// Envelope clock frequency (64 Hz).
const ENVELOPE_CLOCK: u32 = 64;

/// Square channel 2 (no sweep).
pub struct SquareChannel2 {
    /// NR21: Sound length and duty
    sound_length: u8,
    duty_pattern: u8,
    
    /// NR22: Volume envelope
    initial_volume: u8,
    envelope_direction: bool,
    envelope_period: u8,
    
    /// NR23/NR24: Frequency
    frequency_low: u8,
    frequency_high: u8,
    
    /// NR24: Control
    trigger: bool,
    length_enable: bool,
    
    /// Internal state
    enabled: bool,
    playing: bool,
    
    /// Frequency counter
    frequency_counter: u32,
    frequency_period: u32,
    
    /// Length counter
    length_counter: u16,
    length_counter_enabled: bool,
    
    /// Envelope
    envelope_counter: u32,
    envelope_volume: u8,
    
    /// Wave position
    wave_position: u8,
    
    /// Sample counter
    sample_counter: u32,
}

impl Default for SquareChannel2 {
    fn default() -> Self {
        Self::new()
    }
}

impl SquareChannel2 {
    /// Create a new channel 2.
    pub fn new() -> Self {
        Self {
            sound_length: 0,
            duty_pattern: 0,
            initial_volume: 0,
            envelope_direction: false,
            envelope_period: 0,
            frequency_low: 0,
            frequency_high: 0,
            trigger: false,
            length_enable: false,
            enabled: false,
            playing: false,
            frequency_counter: 0,
            frequency_period: 0,
            length_counter: 0,
            length_counter_enabled: false,
            envelope_counter: 0,
            envelope_volume: 0,
            wave_position: 0,
            sample_counter: 0,
        }
    }
    
    /// Reset the channel.
    pub fn reset(&mut self) {
        self.enabled = false;
        self.playing = false;
        self.frequency_counter = 0;
        self.length_counter = 0;
        self.envelope_counter = 0;
        self.wave_position = 0;
        self.sample_counter = 0;
    }
    
    /// Check if channel is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    
    /// Set channel enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.playing = false;
        }
    }
    
    /// Read NR21.
    pub fn read_nr21(&self) -> u8 {
        (self.duty_pattern << 6) | 0x3F | self.sound_length
    }
    
    /// Write NR21.
    pub fn write_nr21(&mut self, value: u8) {
        self.duty_pattern = (value >> 6) & 0x03;
        self.sound_length = value & 0x3F;
    }
    
    /// Read NR22.
    pub fn read_nr22(&self) -> u8 {
        (self.initial_volume << 4) | 
        (self.envelope_direction as u8) << 3 | 
        self.envelope_period
    }
    
    /// Write NR22.
    pub fn write_nr22(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.envelope_direction = (value & 0x08) != 0;
        self.envelope_period = value & 0x07;
    }
    
    /// Read NR23.
    pub fn read_nr23(&self) -> u8 {
        self.frequency_low
    }
    
    /// Write NR23.
    pub fn write_nr23(&mut self, value: u8) {
        self.frequency_low = value;
        self.update_frequency();
    }
    
    /// Read NR24.
    pub fn read_nr24(&self) -> u8 {
        (self.frequency_high & 0x07) | 
        (self.trigger as u8) << 2 | 
        (self.length_enable as u8) << 6 | 
        0x40
    }
    
    /// Write NR24.
    pub fn write_nr24(&mut self, value: u8) {
        self.frequency_high = value & 0x07;
        self.trigger = (value & 0x80) != 0;
        self.length_enable = (value & 0x40) != 0;
        
        self.update_frequency();
        
        if self.trigger {
            self.trigger_channel();
        }
    }
    
    /// Update frequency from registers.
    fn update_frequency(&mut self) {
        let freq = (self.frequency_high as u16) << 8 | self.frequency_low as u16;
        self.frequency_period = ((2048 - freq) as u32) * 4 * CHANNEL2_FREQUENCY_DIVISOR;
    }
    
    /// Trigger the channel.
    fn trigger_channel(&mut self) {
        self.playing = true;
        self.wave_position = 0;
        self.envelope_volume = self.initial_volume;
        self.envelope_counter = 0;
        
        if self.length_counter_enabled && self.length_counter == 0 {
            self.length_counter = 64 - self.sound_length as u16;
        }
    }
    
    /// Clock the channel.
    pub fn clock(&mut self) {
        if !self.enabled || !self.playing {
            return;
        }
        
        self.frequency_counter += 1;
        if self.frequency_counter >= self.frequency_period {
            self.frequency_counter = 0;
            self.wave_position = (self.wave_position + 1) & 0x07;
        }
        
        if self.length_counter_enabled {
            self.sample_counter += 1;
            if self.sample_counter >= CPU_CLOCK / LENGTH_CLOCK {
                self.sample_counter = 0;
                if self.length_counter > 0 {
                    self.length_counter -= 1;
                    if self.length_counter == 0 {
                        self.playing = false;
                    }
                }
            }
        }
        
        if self.envelope_period > 0 {
            if self.sample_counter >= CPU_CLOCK / ENVELOPE_CLOCK {
                self.sample_counter = 0;
                if self.envelope_direction {
                    if self.envelope_volume < 15 {
                        self.envelope_volume += 1;
                    }
                } else {
                    if self.envelope_volume > 0 {
                        self.envelope_volume -= 1;
                    }
                }
            }
        }
    }
    
    /// Get the current sample.
    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.playing {
            return 0.0;
        }
        
        let duty = DUTY_PATTERNS[self.duty_pattern as usize];
        let sample = duty[self.wave_position as usize] as f32;
        
        sample * (self.envelope_volume as f32 / 15.0)
    }
}
