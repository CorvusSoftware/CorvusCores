// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Channel 3: Wave channel (custom waveforms).
//!
//! Registers:
//! - NR30: Sound on/off (FF1A)
//! - NR31: Sound length (FF1B)
//! - NR32: Volume (FF1C)
//! - NR33: Frequency low (FF1D)
//! - NR34: Frequency high, trigger, length enable (FF1E)
//!
//! Wave RAM: FF30-FF3F (16 bytes, 32 samples per wave)

use super::CPU_CLOCK;

/// Wave channel frequency divisor.
const FREQUENCY_DIVISOR: u32 = 2;

/// Length counter clock frequency (256 Hz).
const LENGTH_CLOCK: u32 = 256;

/// Wave RAM size in bytes.
const WAVE_RAM_SIZE: usize = 16;

/// Wave channel.
pub struct WaveChannel {
    /// NR30: Sound on/off
    sound_on: bool,
    
    /// NR31: Sound length
    sound_length: u8,
    
    /// NR32: Volume
    volume: u8,
    
    /// NR33/NR34: Frequency
    frequency_low: u8,
    frequency_high: u8,
    
    /// NR34: Control
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
    
    /// Wave RAM
    wave_ram: [u8; WAVE_RAM_SIZE],
    
    /// Wave position (0-31)
    wave_position: u8,
    
    /// Wave index (0-1 for 4-bit samples, 0-3 for 2-bit samples)
    wave_index: u8,
    
    /// Sample counter
    sample_counter: u32,
}

impl Default for WaveChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveChannel {
    /// Create a new wave channel.
    pub fn new() -> Self {
        Self {
            sound_on: false,
            sound_length: 0,
            volume: 0,
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
            wave_ram: [0; WAVE_RAM_SIZE],
            wave_position: 0,
            wave_index: 0,
            sample_counter: 0,
        }
    }
    
    /// Reset the channel.
    pub fn reset(&mut self) {
        self.enabled = false;
        self.playing = false;
        self.frequency_counter = 0;
        self.length_counter = 0;
        self.wave_position = 0;
        self.wave_index = 0;
        self.sample_counter = 0;
    }
    
    /// Check if channel is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled && self.sound_on
    }
    
    /// Set channel enabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.playing = false;
        }
    }
    
    /// Read NR30.
    pub fn read_nr30(&self) -> u8 {
        (self.sound_on as u8) << 7 | 0x7F
    }
    
    /// Write NR30.
    pub fn write_nr30(&mut self, value: u8) {
        self.sound_on = (value & 0x80) != 0;
    }
    
    /// Read NR31.
    pub fn read_nr31(&self) -> u8 {
        0xFF | self.sound_length
    }
    
    /// Write NR31.
    pub fn write_nr31(&mut self, value: u8) {
        self.sound_length = value;
    }
    
    /// Read NR32.
    pub fn read_nr32(&self) -> u8 {
        (self.volume << 5) | 0x9F
    }
    
    /// Write NR32.
    pub fn write_nr32(&mut self, value: u8) {
        self.volume = (value >> 5) & 0x03;
    }
    
    /// Read NR33.
    pub fn read_nr33(&self) -> u8 {
        self.frequency_low
    }
    
    /// Write NR33.
    pub fn write_nr33(&mut self, value: u8) {
        self.frequency_low = value;
        self.update_frequency();
    }
    
    /// Read NR34.
    pub fn read_nr34(&self) -> u8 {
        (self.frequency_high & 0x07) | 
        (self.trigger as u8) << 2 | 
        (self.length_enable as u8) << 6 | 
        0x40
    }
    
    /// Write NR34.
    pub fn write_nr34(&mut self, value: u8) {
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
        self.frequency_period = ((2048 - freq) as u32) * 2 * FREQUENCY_DIVISOR;
    }
    
    /// Trigger the channel.
    fn trigger_channel(&mut self) {
        self.playing = true;
        self.wave_position = 0;
        self.wave_index = 0;
        
        if self.length_counter_enabled && self.length_counter == 0 {
            self.length_counter = 256 - self.sound_length as u16;
        }
    }
    
    /// Read from wave RAM.
    pub fn read_wave_ram(&self, address: u16) -> Option<u8> {
        if address >= 0xFF30 && address <= 0xFF3F {
            Some(self.wave_ram[(address - 0xFF30) as usize])
        } else {
            None
        }
    }
    
    /// Write to wave RAM.
    pub fn write_wave_ram(&mut self, address: u16, value: u8) {
        if address >= 0xFF30 && address <= 0xFF3F {
            self.wave_ram[(address - 0xFF30) as usize] = value;
        }
    }
    
    /// Clock the channel.
    pub fn clock(&mut self) {
        if !self.enabled || !self.sound_on || !self.playing {
            return;
        }
        
        self.frequency_counter += 1;
        if self.frequency_counter >= self.frequency_period {
            self.frequency_counter = 0;
            self.wave_position = (self.wave_position + 1) & 0x1F;
            
            if self.wave_position & 0x01 == 0 {
                self.wave_index = (self.wave_index + 1) & 0x0F;
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
    }
    
    /// Get the current sample.
    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.sound_on || !self.playing {
            return 0.0;
        }
        
        let wave_byte = self.wave_ram[self.wave_index as usize];
        let sample = if self.volume == 0 {
            0.0
        } else if self.volume == 1 {
            ((wave_byte >> 6) & 0x03) as f32 / 3.0
        } else {
            (wave_byte >> 4) as f32 / 15.0
        };
        
        sample * (self.volume as f32 / 3.0)
    }
}
