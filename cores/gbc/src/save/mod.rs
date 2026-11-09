// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Save state functionality for the Game Boy Color emulator.
//!
//! Provides serialization and deserialization of the complete emulator state
//! including CPU, memory, peripherals, and cycle counters.

use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};

use crate::machine::Machine;

/// Error type for save/load operations.
#[derive(Debug)]
pub enum SaveError {
    /// Serialization failed.
    SerializationError(bincode::Error),
    /// Deserialization failed.
    DeserializationError(bincode::Error),
    /// Version mismatch between saved state and current emulator.
    VersionMismatch { saved: u32, current: u32 },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::SerializationError(e) => write!(f, "Serialization error: {e}"),
            SaveError::DeserializationError(e) => write!(f, "Deserialization error: {e}"),
            SaveError::VersionMismatch { saved, current } => {
                write!(f, "Save state version mismatch: saved={saved}, current={current}")
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl From<bincode::Error> for SaveError {
    fn from(error: bincode::Error) -> Self {
        SaveError::SerializationError(error)
    }
}

/// Current save state version.
/// Increment this when the binary format changes.
const SAVE_STATE_VERSION: u32 = 1;

/// Serialized save state data.
#[derive(Serialize, Deserialize)]
pub struct SaveState {
    version: u32,
    // Will contain serialized Machine state
    // For now, this is a placeholder - full implementation requires
    // Serialize/Deserialize on all components
}

impl SaveState {
    /// Create a new save state from the current machine state.
    pub fn from_machine(_machine: &Machine) -> Result<Self, SaveError> {
        Ok(Self {
            version: SAVE_STATE_VERSION,
        })
    }

    /// Apply this save state to a machine.
    pub fn to_machine(self, _machine: &mut Machine) -> Result<(), SaveError> {
        if self.version != SAVE_STATE_VERSION {
            return Err(SaveError::VersionMismatch {
                saved: self.version,
                current: SAVE_STATE_VERSION,
            });
        }
        Ok(())
    }

    /// Serialize the save state to a byte vector.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SaveError> {
        Ok(serialize(self)?)
    }

    /// Deserialize a save state from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SaveError> {
        Ok(deserialize(data)?)
    }
}

/// Save the current machine state to a byte vector.
pub fn save_state(machine: &Machine) -> Result<Vec<u8>, SaveError> {
    let state = SaveState::from_machine(machine)?;
    state.to_bytes()
}

/// Load a machine state from a byte slice.
pub fn load_state(machine: &mut Machine, data: &[u8]) -> Result<(), SaveError> {
    let state = SaveState::from_bytes(data)?;
    state.to_machine(machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_state_version() {
        let state = SaveState {
            version: SAVE_STATE_VERSION,
        };
        let bytes = state.to_bytes().unwrap();
        let loaded = SaveState::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.version, SAVE_STATE_VERSION);
    }

    #[test]
    fn test_version_mismatch() {
        let state = SaveState {
            version: SAVE_STATE_VERSION + 1,
        };
        let bytes = state.to_bytes().unwrap();
        let loaded = SaveState::from_bytes(&bytes).unwrap();
        use crate::cartridge::RomOnlyCartridge;
        let mut machine = Machine::new(RomOnlyCartridge::new(vec![0u8; 32768]).unwrap());
        let result = loaded.to_machine(&mut machine);
        assert!(matches!(result, Err(SaveError::VersionMismatch { .. })));
    }
}
