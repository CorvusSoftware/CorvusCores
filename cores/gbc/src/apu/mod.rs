// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy Audio Processing Unit (APU) emulation.
//!
//! The APU contains four sound channels:
//! - Channel 1: Square wave with sweep
//! - Channel 2: Square wave
//! - Channel 3: Wave channel (custom waveforms)
//! - Channel 4: Noise channel
//!
//! Each channel has:
//! - Frequency generator
//! - Volume envelope
//! - Length counter
//! - Output level control
//!
//! The APU is clocked at 4.194304 MHz and produces stereo audio output.

pub mod channel1;
pub mod channel2;
pub mod channel3;
pub mod channel4;

use channel1::SquareChannel1;
use channel2::SquareChannel2;
use channel3::WaveChannel;
use channel4::NoiseChannel;

/// CPU clock frequency in Hz.
pub const CPU_CLOCK: u32 = 4_194_304;

/// APU sample rate in Hz (Game Boy runs at ~48kHz for audio).
const SAMPLE_RATE: u32 = 48000;

/// Number of CPU cycles per audio sample.
const CYCLES_PER_SAMPLE: u32 = CPU_CLOCK / SAMPLE_RATE;

/// APU main structure.
pub struct Apu {
    /// Channel 1: Square with sweep
    channel1: SquareChannel1,
    /// Channel 2: Square
    channel2: SquareChannel2,
    /// Channel 3: Wave
    channel3: WaveChannel,
    /// Channel 4: Noise
    channel4: NoiseChannel,
    
    /// Sound enable flags (NR52)
    enabled: bool,
    
    /// Master volume and mixing (NR50, NR51)
    master_volume_left: u8,
    master_volume_right: u8,
    
    /// Sound output terminals
    sound_terminal_left: bool,
    sound_terminal_right: bool,
    
    /// Audio buffer for SDL2
    audio_buffer: Vec<f32>,
    
    /// Sample counter
    sample_counter: u32,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    /// Create a new APU instance.
    pub fn new() -> Self {
        Self {
            channel1: SquareChannel1::new(),
            channel2: SquareChannel2::new(),
            channel3: WaveChannel::new(),
            channel4: NoiseChannel::new(),
            enabled: false,
            master_volume_left: 0,
            master_volume_right: 0,
            sound_terminal_left: false,
            sound_terminal_right: false,
            audio_buffer: Vec::with_capacity(1024),
            sample_counter: 0,
        }
    }
    
    /// Read from an APU register.
    pub fn read(&self, address: u16) -> Option<u8> {
        match address {
            // Channel 1
            0xFF10 => Some(self.channel1.read_nr10()),
            0xFF11 => Some(self.channel1.read_nr11()),
            0xFF12 => Some(self.channel1.read_nr12()),
            0xFF13 => Some(self.channel1.read_nr13()),
            0xFF14 => Some(self.channel1.read_nr14()),
            
            // Channel 2
            0xFF16 => Some(self.channel2.read_nr21()),
            0xFF17 => Some(self.channel2.read_nr22()),
            0xFF18 => Some(self.channel2.read_nr23()),
            0xFF19 => Some(self.channel2.read_nr24()),
            
            // Channel 3
            0xFF1A => Some(self.channel3.read_nr30()),
            0xFF1B => Some(self.channel3.read_nr31()),
            0xFF1C => Some(self.channel3.read_nr32()),
            0xFF1D => Some(self.channel3.read_nr33()),
            0xFF1E => Some(self.channel3.read_nr34()),
            
            // Channel 4
            0xFF20 => Some(self.channel4.read_nr41()),
            0xFF21 => Some(self.channel4.read_nr42()),
            0xFF22 => Some(self.channel4.read_nr43()),
            0xFF23 => Some(self.channel4.read_nr44()),
            
            // Wave RAM (FF30-FF3F)
            0xFF30..=0xFF3F => self.channel3.read_wave_ram(address),
            
            // Master control
            0xFF24 => Some(self.read_nr50()),
            0xFF25 => Some(self.read_nr51()),
            0xFF26 => Some(self.read_nr52()),
            
            _ => None,
        }
    }
    
    /// Write to an APU register.
    pub fn write(&mut self, address: u16, value: u8) -> bool {
        match address {
            // Channel 1
            0xFF10 => {
                self.channel1.write_nr10(value);
                true
            }
            0xFF11 => {
                self.channel1.write_nr11(value);
                true
            }
            0xFF12 => {
                self.channel1.write_nr12(value);
                true
            }
            0xFF13 => {
                self.channel1.write_nr13(value);
                true
            }
            0xFF14 => {
                self.channel1.write_nr14(value);
                true
            }
            
            // Channel 2
            0xFF16 => {
                self.channel2.write_nr21(value);
                true
            }
            0xFF17 => {
                self.channel2.write_nr22(value);
                true
            }
            0xFF18 => {
                self.channel2.write_nr23(value);
                true
            }
            0xFF19 => {
                self.channel2.write_nr24(value);
                true
            }
            
            // Channel 3
            0xFF1A => {
                self.channel3.write_nr30(value);
                true
            }
            0xFF1B => {
                self.channel3.write_nr31(value);
                true
            }
            0xFF1C => {
                self.channel3.write_nr32(value);
                true
            }
            0xFF1D => {
                self.channel3.write_nr33(value);
                true
            }
            0xFF1E => {
                self.channel3.write_nr34(value);
                true
            }
            
            // Channel 4
            0xFF20 => {
                self.channel4.write_nr41(value);
                true
            }
            0xFF21 => {
                self.channel4.write_nr42(value);
                true
            }
            0xFF22 => {
                self.channel4.write_nr43(value);
                true
            }
            0xFF23 => {
                self.channel4.write_nr44(value);
                true
            }
            
            // Wave RAM (FF30-FF3F)
            0xFF30..=0xFF3F => {
                self.channel3.write_wave_ram(address, value);
                true
            }
            
            // Master control
            0xFF24 => {
                self.write_nr50(value);
                true
            }
            0xFF25 => {
                self.write_nr51(value);
                true
            }
            0xFF26 => {
                self.write_nr52(value);
                true
            }
            
            _ => false,
        }
    }
    
    /// Read NR50 (Master volume and VIN mixing)
    fn read_nr50(&self) -> u8 {
        (self.master_volume_left << 4) | self.master_volume_right | 0x80
    }
    
    /// Write NR50
    fn write_nr50(&mut self, value: u8) {
        self.master_volume_left = (value >> 4) & 0x07;
        self.master_volume_right = value & 0x07;
    }
    
    /// Read NR51 (Sound terminal selection)
    fn read_nr51(&self) -> u8 {
        let mut value = 0x80;
        if self.sound_terminal_left {
            value |= 0x10;
        }
        if self.sound_terminal_right {
            value |= 0x01;
        }
        value
    }
    
    /// Write NR51
    fn write_nr51(&mut self, value: u8) {
        self.sound_terminal_left = (value & 0x10) != 0;
        self.sound_terminal_right = (value & 0x01) != 0;
    }
    
    /// Read NR52 (Sound on/off)
    fn read_nr52(&self) -> u8 {
        let mut value = 0x80 | 0x40; // Bits 7 and 6 are always 1
        if self.enabled {
            value |= 0x80;
        }
        // Bit 0-3: Channel enable flags
        value |= self.channel1.enabled() as u8;
        value |= (self.channel2.enabled() as u8) << 1;
        value |= (self.channel3.enabled() as u8) << 2;
        value |= (self.channel4.enabled() as u8) << 3;
        value
    }
    
    /// Write NR52
    fn write_nr52(&mut self, value: u8) {
        self.enabled = (value & 0x80) != 0;
        
        // Enable/disable channels
        self.channel1.set_enabled((value & 0x01) != 0);
        self.channel2.set_enabled((value & 0x02) != 0);
        self.channel3.set_enabled((value & 0x04) != 0);
        self.channel4.set_enabled((value & 0x08) != 0);
        
        // If APU is disabled, reset all channels
        if !self.enabled {
            self.channel1.reset();
            self.channel2.reset();
            self.channel3.reset();
            self.channel4.reset();
        }
    }
    
    /// Clock the APU by one CPU cycle.
    pub fn clock(&mut self) {
        self.sample_counter += 1;
        
        if self.sample_counter >= CYCLES_PER_SAMPLE {
            self.sample_counter = 0;
            self.generate_sample();
        }
        
        // Clock all channels
        self.channel1.clock();
        self.channel2.clock();
        self.channel3.clock();
        self.channel4.clock();
    }
    
    /// Generate one audio sample.
    fn generate_sample(&mut self) {
        if !self.enabled {
            self.audio_buffer.push(0.0);
            self.audio_buffer.push(0.0);
            return;
        }
        
        // Get samples from each channel
        let ch1_sample = self.channel1.get_sample();
        let ch2_sample = self.channel2.get_sample();
        let ch3_sample = self.channel3.get_sample();
        let ch4_sample = self.channel4.get_sample();
        
        // Mix channels
        let left_sample = ch1_sample + ch2_sample + ch3_sample + ch4_sample;
        let right_sample = left_sample; // Mono for now
        
        // Apply master volume
        let left_vol = self.master_volume_left as f32 / 7.0;
        let right_vol = self.master_volume_right as f32 / 7.0;
        
        let left_output = left_sample * left_vol;
        let right_output = right_sample * right_vol;
        
        self.audio_buffer.push(left_output);
        self.audio_buffer.push(right_output);
    }
    
    /// Get the audio buffer and clear it.
    pub fn get_audio_buffer(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.audio_buffer)
    }
    
    /// Check if audio data is available.
    pub fn has_audio(&self) -> bool {
        !self.audio_buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apu_initial_state() {
        let apu = Apu::new();
        assert!(!apu.enabled);
        assert_eq!(apu.read_nr52(), 0x80 | 0x40); // Bits 7 and 6 always set
    }

    #[test]
    fn test_apu_enable() {
        let mut apu = Apu::new();
        apu.write_nr52(0x80);
        assert!(apu.enabled);
    }

    #[test]
    fn test_apu_disable() {
        let mut apu = Apu::new();
        apu.write_nr52(0x00);
        assert!(!apu.enabled);
    }

    #[test]
    fn test_nr50_read_write() {
        let mut apu = Apu::new();
        apu.write_nr50(0x77);
        assert_eq!(apu.read_nr50(), 0xF7); // 0x80 | 0x77
    }

    #[test]
    fn test_nr51_read_write() {
        let mut apu = Apu::new();
        apu.write_nr51(0x11);
        assert_eq!(apu.read_nr51(), 0x91); // 0x80 | 0x11
    }
}
