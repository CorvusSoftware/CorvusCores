// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Work RAM and its CGB bank register.
//!
//! C000–CFFF always maps bank 0. D000–DFFF maps bank 1 in non-CGB
//! mode, or a selectable bank 1–7 in CGB mode. E000–FDFF mirrors
//! C000–DDFF, with the same currently selected bank.

pub(crate) struct WorkRam {
    banks: [[u8; 0x1000]; 8],
    bank_register: u8,
    cgb_mode: bool,
}

impl WorkRam {
    pub(crate) fn new(cgb_mode: bool) -> Self {
        Self {
            // Deterministic starting data, not a simulated power-on state.
            banks: [[0; 0x1000]; 8],
            bank_register: 1,
            cgb_mode,
        }
    }

    fn index(&self, address: u16) -> Option<(usize, usize)> {
        let effective = match address {
            0xC000..=0xDFFF => address,
            0xE000..=0xFDFF => address - 0x2000,
            _ => return None,
        };
        let bank = if effective < 0xD000 {
            0
        } else if self.cgb_mode {
            usize::from(self.bank_register.max(1))
        } else {
            1
        };
        Some((bank, usize::from(effective & 0x0FFF)))
    }

    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        let (bank, offset) = self.index(address)?;
        Some(self.banks[bank][offset])
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) -> bool {
        let Some((bank, offset)) = self.index(address) else {
            return false;
        };
        self.banks[bank][offset] = value;
        true
    }

    pub(crate) fn read_bank_register(&self) -> Option<u8> {
        self.cgb_mode.then_some(0xF8 | self.bank_register)
    }

    pub(crate) fn write_bank_register(&mut self, value: u8) -> bool {
        if !self.cgb_mode {
            return false;
        }
        self.bank_register = value & 0x07;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::WorkRam;

    #[test]
    fn bank_zero_is_fixed_and_switchable_banks_are_independent() {
        let mut wram = WorkRam::new(true);
        assert!(wram.write(0xC001, 0xAB));
        assert!(wram.write(0xD001, 0x11));
        assert!(wram.write_bank_register(7));
        assert!(wram.write(0xD001, 0x77));

        assert_eq!(wram.read(0xC001), Some(0xAB));
        assert_eq!(wram.read(0xD001), Some(0x77));
        assert!(wram.write_bank_register(1));
        assert_eq!(wram.read(0xD001), Some(0x11));
        assert!(wram.write_bank_register(7));
        assert_eq!(wram.read(0xD001), Some(0x77));
    }

    #[test]
    fn selecting_bank_zero_aliases_bank_one_but_register_keeps_zero() {
        let mut wram = WorkRam::new(true);
        assert!(wram.write(0xD000, 0x55));
        assert!(wram.write_bank_register(0));
        assert_eq!(wram.read_bank_register(), Some(0xF8));
        assert_eq!(wram.read(0xD000), Some(0x55));
        assert!(wram.write_bank_register(0xFF));
        assert_eq!(wram.read_bank_register(), Some(0xFF));
    }

    #[test]
    fn echo_mirrors_only_c000_through_ddff() {
        let mut wram = WorkRam::new(true);
        assert!(wram.write(0xE000, 0x12));
        assert_eq!(wram.read(0xC000), Some(0x12));
        assert!(wram.write(0xFDFF, 0x34));
        assert_eq!(wram.read(0xDDFF), Some(0x34));
        assert!(wram.write(0xDE00, 0x56));
        assert_eq!(wram.read(0xFE00), None);
        assert_eq!(wram.read(0xBFFF), None);
        assert_eq!(wram.read(0xDFFF), Some(0x00));
    }

    #[test]
    fn echo_tracks_the_selected_bank() {
        let mut wram = WorkRam::new(true);
        assert!(wram.write(0xF000, 0x11));
        assert!(wram.write_bank_register(2));
        assert!(wram.write(0xF000, 0x22));
        assert_eq!(wram.read(0xD000), Some(0x22));
        assert!(wram.write_bank_register(1));
        assert_eq!(wram.read(0xD000), Some(0x11));
    }

    #[test]
    fn non_cgb_mode_uses_bank_one_and_hides_svbk() {
        let mut wram = WorkRam::new(false);
        assert!(wram.write(0xD000, 0x21));
        assert!(!wram.write_bank_register(5));
        assert_eq!(wram.read_bank_register(), None);
        assert_eq!(wram.read(0xD000), Some(0x21));
    }
}
