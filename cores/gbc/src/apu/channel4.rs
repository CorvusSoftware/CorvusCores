// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Channel 4: Noise channel.
//!
//! Registers:
//! - NR41: Sound length (FF20)
//! - NR42: Volume envelope (FF21)
//! - NR43: Polynomial counter parameters (FF22)
//! - NR44: Trigger, length enable (FF23)
//!
//! The noise channel generates random noise using a Linear Feedback Shift Register (LFSR).

use super::CPU_CLOCK;

/// Length counter clock frequency (256 Hz).
const LENGTH_CLOCK: u32 = 256;

/// Envelope clock frequency (64 Hz).
const ENVELOPE_CLOCK: u32 = 64;

/// Noise frequency divisor.
const FREQUENCY_DIVISOR: u32 = 1;

/// Noise channel.
pub struct NoiseChannel {
    /// NR41: Sound length
    sound_length: u8,
    
    /// NR42: Volume envelope
    initial_volume: u8,
    envelope_direction: bool,
    envelope_period: u8,
    
    /// NR43: Polynomial counter parameters
    clock_shift: u8,
    width_mode: bool,
    divisor_code: u8,
    
    /// NR44: Control
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
    
    /// LFSR (15-bit or 7-bit)
    lfsr: u16,
    
    /// Sample counter
    sample_counter: u32,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseChannel {
    /// Create a new noise channel.
    pub fn new() -> Self {
        Self {
            sound_length: 0,
            initial_volume: 0,
            envelope_direction: false,
            envelope_period: 0,
            clock_shift: 0,
            width_mode: false,
            divisor_code: 0,
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
            lfsr: 0,
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
        self.lfsr = 0;
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
    
    /// Read NR41.
    pub fn read_nr41(&self) -> u8 {
        0xFF | self.sound_length
    }
    
    /// Write NR41.
    pub fn write_nr41(&mut self, value: u8) {
        self.sound_length = value & 0x3F;
    }
    
    /// Read NR42.
    pub fn read_nr42(&self) -> u8 {
        (self.initial_volume << 4) | 
        (self.envelope_direction as u8) << 3 | 
        self.envelope_period
    }
    
    /// Write NR42.
    pub fn write_nr42(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.envelope_direction = (value & 0x08) != 0;
        self.envelope_period = value & 0x07;
    }
    
    /// Read NR43.
    pub fn read_nr43(&self) -> u8 {
        (self.clock_shift << 4) | 
        (self.width_mode as u8) << 3 | 
        self.divisor_code | 
        0x00
    }
    
    /// Write NR43.
    pub fn write_nr43(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
        self.update_frequency();
    }
    
    /// Read NR44.
    pub fn read_nr44(&self) -> u8 {
        (self.trigger as u8) << 2 | 
        (self.length_enable as u8) << 6 | 
        0x40
    }
    
    /// Write NR44.
    pub fn write_nr44(&mut self, value: u8) {
        self.trigger = (value & 0x80) != 0;
        self.length_enable = (value & 0x40) != 0;
        
        if self.trigger {
            self.trigger_channel();
        }
    }
    
    /// Update frequency from divisor code.
    fn update_frequency(&mut self) {
        let divisor = match self.divisor_code {
            0 => 2,
            1 => 4,
            2 => 8,
            3 => 16,
            4 => 32,
            5 => 48,
            6 => 64,
            7 => 80,
            _ => 16,
        };
        
        let shift = self.clock_shift as u32;
        self.frequency_period = divisor * (1 << shift) * FREQUENCY_DIVISOR;
    }
    
    /// Trigger the channel.
    fn trigger_channel(&mut self) {
        self.playing = true;
        self.lfsr = if self.width_mode {
            0x40
        } else {
            0x4000
        };
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
            self.clock_lfsr();
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
    
    /// Clock the LFSR.
    fn clock_lfsr(&mut self) {
        let bit0 = self.lfsr & 0x01;
        let bit1 = (self.lfsr >> 1) & 0x01;
        let feedback = bit0 ^ bit1;
        
        if self.width_mode {
            self.lfsr = ((self.lfsr >> 1) | (feedback << 6)) & 0x7F;
        } else {
            self.lfsr = ((self.lfsr >> 1) | (feedback << 14)) & 0x7FFF;
        }
    }
    
    /// Get the current sample.
    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.playing {
            return 0.0;
        }
        
        let output = if self.width_mode {
            ((self.lfsr >> 6) & 0x01) as f32
        } else {
            1.0 - (self.lfsr & 0x01) as f32
        };
        
        output * (self.envelope_volume as f32 / 15.0)
    }
}
