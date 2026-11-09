// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SDL2-based frontend for the GBC emulator.
//!
//! Provides a simple window to display the PPU framebuffer and handle input.
//! Also handles audio output through SDL2's audio subsystem.

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: u32 = 4;
const WINDOW_WIDTH: u32 = SCREEN_WIDTH as u32 * SCALE;
const WINDOW_HEIGHT: u32 = SCREEN_HEIGHT as u32 * SCALE;

/// Audio sample rate (48 kHz).
const AUDIO_SAMPLE_RATE: i32 = 48000;

/// Audio buffer size in samples (stereo).
const AUDIO_BUFFER_SIZE: usize = 1024;

/// Audio callback for SDL2.
/// This is a simple callback that reads from a shared audio buffer.
struct AudioCallbackData {
    audio_buffer: std::sync::Mutex<Vec<f32>>,
}

impl AudioCallback for AudioCallbackData {
    type Channel = f32;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        
        // Fill output with zeros if no data available
        let available = buffer.len();
        if available == 0 {
            for sample in out.iter_mut() {
                *sample = 0.0;
            }
            return;
        }
        
        // Copy available samples
        let samples_to_copy = std::cmp::min(available, out.len());
        out[..samples_to_copy].copy_from_slice(&buffer[..samples_to_copy]);
        
        // Remove consumed samples
        if samples_to_copy > 0 {
            buffer.drain(..samples_to_copy);
        }
        
        // Fill remaining with zeros
        for sample in out[samples_to_copy..].iter_mut() {
            *sample = 0.0;
        }
    }
}

/// SDL2 frontend state.
pub struct Frontend {
    _sdl_context: sdl2::Sdl,
    canvas: sdl2::render::WindowCanvas,
    event_pump: sdl2::EventPump,
    _audio_device: Option<AudioDevice<AudioCallbackData>>,
    audio_data: Option<AudioCallbackData>,
}

impl Frontend {
    /// Create a new SDL2 frontend.
    pub fn new(title: &str) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(
                title,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let canvas = window
            .into_canvas()
            .accelerated()
            .build()
            .map_err(|e| e.to_string())?;

        let event_pump = sdl_context.event_pump()?;
        
        // Initialize audio
        let audio_subsystem = sdl_context.audio()?;
        let (_audio_device, audio_data) = Self::init_audio(audio_subsystem)?;

        Ok(Self {
            _sdl_context: sdl_context,
            canvas,
            event_pump,
            _audio_device,
            audio_data,
        })
    }
    
    /// Initialize SDL2 audio.
    fn init_audio(audio_subsystem: sdl2::AudioSubsystem) -> Result<(Option<AudioDevice<AudioCallbackData>>, Option<AudioCallbackData>), String> {
        let desired_spec = AudioSpecDesired {
            freq: Some(AUDIO_SAMPLE_RATE),
            channels: Some(2), // Stereo
            samples: Some(AUDIO_BUFFER_SIZE as u16),
        };
        
        let audio_data = AudioCallbackData {
            audio_buffer: std::sync::Mutex::new(Vec::with_capacity(AUDIO_BUFFER_SIZE * 2)),
        };
        
        match audio_subsystem.open_playback(None, &desired_spec, |spec| {
            // Recreate the callback with the actual spec
            AudioCallbackData {
                audio_buffer: std::sync::Mutex::new(Vec::with_capacity(spec.size as usize * 2)),
            }
        }) {
            Ok(device) => Ok((Some(device), Some(audio_data))),
            Err(e) => {
                eprintln!("Warning: Could not open audio device: {}", e);
                Ok((None, None))
            }
        }
    }

    /// Update the canvas with the current framebuffer.
    pub fn update_framebuffer(&mut self, framebuffer: &[u16]) {
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();
        
        // Draw each pixel scaled up
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let pixel = framebuffer[y * SCREEN_WIDTH + x];
                // Convert RGB555 to RGB888
                let r = ((pixel >> 10) & 0x1F) as u8 * 8;
                let g = ((pixel >> 5) & 0x1F) as u8 * 8;
                let b = (pixel & 0x1F) as u8 * 8;
                
                let rect = Rect::new(
                    (x as u32 * SCALE) as i32,
                    (y as u32 * SCALE) as i32,
                    SCALE,
                    SCALE,
                );
                
                self.canvas.set_draw_color(Color::RGB(r, g, b));
                let _ = self.canvas.fill_rect(rect);
            }
        }
    }

    /// Render the current frame.
    pub fn render(&mut self) -> Result<(), String> {
        self.canvas.present();
        Ok(())
    }

    /// Add audio samples to the audio buffer.
    pub fn queue_audio(&mut self, samples: Vec<f32>) {
        if let Some(ref audio_data) = self.audio_data {
            let mut buffer = audio_data.audio_buffer.lock().unwrap();
            buffer.extend(samples);
        }
    }

    /// Process SDL events and return joypad state.
    pub fn process_events(&mut self) -> JoypadState {
        let mut state = JoypadState::default();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    state.quit = true;
                }
                Event::KeyDown { keycode: Some(key), .. } => {
                    state = Self::update_joypad_from_key(state, key, true);
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    state = Self::update_joypad_from_key(state, key, false);
                }
                _ => {}
            }
        }

        state
    }

    fn update_joypad_from_key(mut state: JoypadState, key: Keycode, pressed: bool) -> JoypadState {
        match key {
            Keycode::Right => state.right = pressed,
            Keycode::Left => state.left = pressed,
            Keycode::Up => state.up = pressed,
            Keycode::Down => state.down = pressed,
            Keycode::A => state.a = pressed,
            Keycode::S => state.b = pressed,
            Keycode::Z => state.select = pressed,
            Keycode::Return => state.start = pressed,
            Keycode::Escape => state.quit = true,
            _ => {}
        }
        state
    }
}

/// Current joypad input state.
#[derive(Clone, Copy, Debug, Default)]
pub struct JoypadState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub quit: bool,
}
