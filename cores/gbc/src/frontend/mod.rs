// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! SDL2-based frontend for the GBC emulator.
//!
//! Provides a simple window to display the PPU framebuffer and handle input.

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: u32 = 4;
const WINDOW_WIDTH: u32 = SCREEN_WIDTH as u32 * SCALE;
const WINDOW_HEIGHT: u32 = SCREEN_HEIGHT as u32 * SCALE;

/// SDL2 frontend state.
pub struct Frontend {
    _sdl_context: sdl2::Sdl,
    canvas: sdl2::render::WindowCanvas,
    event_pump: sdl2::EventPump,
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

        Ok(Self {
            _sdl_context: sdl_context,
            canvas,
            event_pump,
        })
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
