// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Channel 1: Square wave with sweep.
//!
//! Registers:
//! - NR10: Sweep period, negate, shift (FF10)
//! - NR11: Sound length, wave pattern duty (FF11)
//! - NR12: Volume envelope (FF12)
//! - NR13: Frequency low (FF13)
//! - NR14: Frequency high, trigger, length enable (FF14)

use super::CPU_CLOCK;

/// Square wave duty patterns (2-bit index).
const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

/// Frequency counter divisor for channel 1.
const FREQUENCY_DIVISOR: u32 = 1;

/// Sweep period divisor.
const SWEEP_DIVISOR: u32 = 128;

/// Length counter clock frequency (256 Hz).
const LENGTH_CLOCK: u32 = 256;

/// Envelope clock frequency (64 Hz).
const ENVELOPE_CLOCK: u32 = 64;

/// Square channel 1 (with sweep).
pub struct SquareChannel1 {
    /// NR10: Sweep register
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    
    /// NR11: Sound length and duty
    sound_length: u8,
    duty_pattern: u8,
    
    /// NR12: Volume envelope
    initial_volume: u8,
    envelope_direction: bool,
    envelope_period: u8,
    
    /// NR13/NR14: Frequency
    frequency_low: u8,
    frequency_high: u8,
    
    /// NR14: Control
    trigger: bool,
    length_enable: bool,
    
    /// Internal state
    enabled: bool,
    playing: bool,
    
    /// Frequency counter
    frequency_counter: u32,
    frequency_period: u32,
    
    /// Sweep counter
    sweep_counter: u32,
    sweep_enabled: bool,
    shadow_frequency: u16,
    
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

impl Default for SquareChannel1 {
    fn default() -> Self {
        Self::new()
    }
}

impl SquareChannel1 {
    /// Create a new channel 1.
    pub fn new() -> Self {
        Self {
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
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
            sweep_counter: 0,
            sweep_enabled: false,
            shadow_frequency: 0,
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
        self.sweep_counter = 0;
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
    
    /// Read NR10.
    pub fn read_nr10(&self) -> u8 {
        (self.sweep_period << 4) | 
        (self.sweep_negate as u8) << 3 | 
        self.sweep_shift
    }
    
    /// Write NR10.
    pub fn write_nr10(&mut self, value: u8) {
        self.sweep_period = (value >> 4) & 0x07;
        self.sweep_negate = (value & 0x08) != 0;
        self.sweep_shift = value & 0x07;
    }
    
    /// Read NR11.
    pub fn read_nr11(&self) -> u8 {
        (self.duty_pattern << 6) | 0x3F | self.sound_length
    }
    
    /// Write NR11.
    pub fn write_nr11(&mut self, value: u8) {
        self.duty_pattern = (value >> 6) & 0x03;
        self.sound_length = value & 0x3F;
    }
    
    /// Read NR12.
    pub fn read_nr12(&self) -> u8 {
        (self.initial_volume << 4) | 
        (self.envelope_direction as u8) << 3 | 
        self.envelope_period
    }
    
    /// Write NR12.
    pub fn write_nr12(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.envelope_direction = (value & 0x08) != 0;
        self.envelope_period = value & 0x07;
    }
    
    /// Read NR13.
    pub fn read_nr13(&self) -> u8 {
        self.frequency_low
    }
    
    /// Write NR13.
    pub fn write_nr13(&mut self, value: u8) {
        self.frequency_low = value;
        self.update_frequency();
    }
    
    /// Read NR14.
    pub fn read_nr14(&self) -> u8 {
        (self.frequency_high & 0x07) | 
        (self.trigger as u8) << 2 | 
        (self.length_enable as u8) << 6 | 
        0x40
    }
    
    /// Write NR14.
    pub fn write_nr14(&mut self, value: u8) {
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
        self.frequency_period = ((2048 - freq) as u32) * 4 * FREQUENCY_DIVISOR;
    }
    
    /// Trigger the channel.
    fn trigger_channel(&mut self) {
        self.playing = true;
        self.wave_position = 0;
        self.envelope_volume = self.initial_volume;
        self.envelope_counter = 0;
        self.shadow_frequency = (self.frequency_high as u16) << 8 | self.frequency_low as u16;
        
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
        
        self.sweep_counter += 1;
        if self.sweep_counter >= SWEEP_DIVISOR {
            self.sweep_counter = 0;
            if self.sweep_enabled && self.sweep_period > 0 {
                self.update_sweep();
            }
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
    
    /// Update sweep frequency.
    fn update_sweep(&mut self) {
        let shift = self.sweep_shift;
        if shift == 0 {
            return;
        }
        
        let delta = self.shadow_frequency >> shift;
        let new_freq = if self.sweep_negate {
            self.shadow_frequency.wrapping_sub(delta)
        } else {
            self.shadow_frequency.wrapping_add(delta)
        };
        
        if new_freq >= 2048 {
            self.playing = false;
        } else {
            self.shadow_frequency = new_freq;
            self.frequency_high = (new_freq >> 8) as u8 & 0x07;
            self.frequency_low = new_freq as u8;
            self.update_frequency();
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
