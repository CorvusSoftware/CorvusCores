//! Two banks of 8 KiB video RAM; VBK selects the CPU-visible bank.

pub(crate) struct VideoRam {
    banks: [[u8; 0x2000]; 2],
    selected_bank: u8,
    cgb_mode: bool,
}

impl VideoRam {
    pub(crate) fn new(cgb_mode: bool) -> Self {
        Self {
            banks: [[0; 0x2000]; 2],
            selected_bank: 0,
            cgb_mode,
        }
    }

    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        let offset = address.checked_sub(0x8000)?;
        if offset >= 0x2000 {
            return None;
        }
        Some(self.banks[usize::from(self.selected_bank)][usize::from(offset)])
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) -> bool {
        let Some(offset) = address.checked_sub(0x8000) else {
            return false;
        };
        if offset >= 0x2000 {
            return false;
        }
        self.banks[usize::from(self.selected_bank)][usize::from(offset)] = value;
        true
    }

    pub(crate) fn read_bank_register(&self) -> Option<u8> {
        self.cgb_mode.then_some(0xFE | self.selected_bank)
    }

    pub(crate) fn write_bank_register(&mut self, value: u8) -> bool {
        if !self.cgb_mode {
            return false;
        }
        self.selected_bank = value & 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::VideoRam;

    #[test]
    fn color_mode_keeps_two_independent_banks() {
        let mut vram = VideoRam::new(true);
        assert!(vram.write(0x8000, 0x12));
        assert!(vram.write(0x9FFF, 0x34));
        assert!(vram.write_bank_register(1));
        assert_eq!(vram.read_bank_register(), Some(0xFF));
        assert_eq!(vram.read(0x8000), Some(0));
        assert!(vram.write(0x8000, 0x56));
        assert!(vram.write_bank_register(0));
        assert_eq!(vram.read_bank_register(), Some(0xFE));
        assert_eq!(vram.read(0x8000), Some(0x12));
        assert_eq!(vram.read(0x9FFF), Some(0x34));
        assert!(vram.write_bank_register(1));
        assert_eq!(vram.read(0x8000), Some(0x56));
    }

    #[test]
    fn non_cgb_mode_remains_in_bank_zero() {
        let mut vram = VideoRam::new(false);
        assert!(vram.write(0x9FFF, 0x77));
        assert!(!vram.write_bank_register(1));
        assert_eq!(vram.read_bank_register(), None);
        assert_eq!(vram.read(0x9FFF), Some(0x77));
    }

    #[test]
    fn refuses_addresses_outside_vram() {
        let mut vram = VideoRam::new(true);
        assert_eq!(vram.read(0x7FFF), None);
        assert_eq!(vram.read(0xA000), None);
        assert!(!vram.write(0xA000, 0x10));
    }
}
