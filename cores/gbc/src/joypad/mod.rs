// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy joypad input handling.
//!
//! The joypad is memory-mapped at FF00 and provides button state reading.

use crate::frontend::JoypadState;

/// Button masks for P1 register.
/// P1 register format (read):
///   Bit 0: A (0 = pressed)
///   Bit 1: B (0 = pressed)
///   Bit 2: Select (0 = pressed)
///   Bit 3: Start (0 = pressed)
///   Bit 4: Right (0 = pressed)
///   Bit 5: Left (0 = pressed)
///   Bit 6: Up (0 = pressed)
///   Bit 7: Down (0 = pressed)
///
/// To read, first write to select which lines:
///   Write 0x10 to select direction keys (read bits 4-5)
///   Write 0x20 to select button keys (read bits 0-3)
///   Write 0x30 to select neither (all bits read as 1)
///   Write 0x00 to select both

const BUTTON_A_MASK: u8 = 0x01;
const BUTTON_B_MASK: u8 = 0x02;
const BUTTON_SELECT_MASK: u8 = 0x04;
const BUTTON_START_MASK: u8 = 0x08;
const BUTTON_RIGHT_MASK: u8 = 0x10;
const BUTTON_LEFT_MASK: u8 = 0x20;
const BUTTON_UP_MASK: u8 = 0x40;
const BUTTON_DOWN_MASK: u8 = 0x80;

/// Joypad hardware state.
pub struct Joypad {
    /// The current value written to P1 register (controls which lines are selected)
    p1_register: u8,
    /// External input state
    external_state: JoypadState,
}

impl Default for Joypad {
    fn default() -> Self {
        Self {
            p1_register: 0x30, // Default: no lines selected
            external_state: JoypadState::default(),
        }
    }
}

impl Joypad {
    /// Create a new joypad.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update external input state.
    pub fn update_state(&mut self, state: JoypadState) {
        self.external_state = state;
    }

    /// Read from joypad register (FF00).
    pub fn read(&self) -> u8 {
        let mut value = 0xFF;
        
        // If direction keys are selected (bit 4 = 0)
        if self.p1_register & 0x10 == 0 {
            if self.external_state.right {
                value &= !BUTTON_RIGHT_MASK;
            }
            if self.external_state.left {
                value &= !BUTTON_LEFT_MASK;
            }
            if self.external_state.up {
                value &= !BUTTON_UP_MASK;
            }
            if self.external_state.down {
                value &= !BUTTON_DOWN_MASK;
            }
        } else {
            // Direction keys not selected, set bits 4-7 to 1
            value |= 0xF0;
        }
        
        // If button keys are selected (bit 5 = 0)
        if self.p1_register & 0x20 == 0 {
            if self.external_state.a {
                value &= !BUTTON_A_MASK;
            }
            if self.external_state.b {
                value &= !BUTTON_B_MASK;
            }
            if self.external_state.select {
                value &= !BUTTON_SELECT_MASK;
            }
            if self.external_state.start {
                value &= !BUTTON_START_MASK;
            }
        } else {
            // Button keys not selected, set bits 0-3 to 1
            value |= 0x0F;
        }
        
        // Bits 4-5 should echo what was written to P1
        value = (value & 0xCF) | (self.p1_register & 0x30);
        
        value
    }

    /// Write to joypad register (FF00).
    pub fn write(&mut self, value: u8) {
        // Only bits 4-5 are writable
        self.p1_register = (self.p1_register & 0x0F) | (value & 0x30);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joypad_read_all_released() {
        let mut joypad = Joypad::new();
        // With no lines selected (bits 4-5 = 11), all should read as 1
        assert_eq!(joypad.read(), 0xFF);
    }

    #[test]
    fn test_joypad_read_button_pressed() {
        let mut joypad = Joypad::new();
        joypad.update_state(JoypadState {
            a: true,
            ..Default::default()
        });
        // Select button lines (bit 5 = 0)
        joypad.write(0x10);
        // Button A should be 0 (pressed), others 1
        let result = joypad.read();
        assert_eq!(result & BUTTON_A_MASK, 0);
        assert_eq!(result & BUTTON_B_MASK, BUTTON_B_MASK);
    }

    #[test]
    fn test_joypad_read_direction_pressed() {
        let mut joypad = Joypad::new();
        joypad.update_state(JoypadState {
            right: true,
            ..Default::default()
        });
        // Select direction lines (bit 4 = 0)
        joypad.write(0x20);
        // Right button should be 0 (pressed)
        let result = joypad.read();
        assert_eq!(result & BUTTON_RIGHT_MASK, 0);
        assert_eq!(result & BUTTON_LEFT_MASK, BUTTON_LEFT_MASK);
    }

    #[test]
    fn test_joypad_default_state() {
        let joypad = Joypad::new();
        // Default should have no lines selected
        assert_eq!(joypad.read(), 0xFF);
    }
}
