//! CPU high RAM (FF80–FFFE). FFFF is an interrupt register, not HRAM.

pub(crate) struct HighRam {
    bytes: [u8; 0x7F],
}

impl HighRam {
    pub(crate) fn new() -> Self {
        Self { bytes: [0; 0x7F] }
    }

    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        let offset = address.checked_sub(0xFF80)?;
        self.bytes.get(usize::from(offset)).copied()
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) -> bool {
        let Some(offset) = address.checked_sub(0xFF80) else {
            return false;
        };
        let Some(slot) = self.bytes.get_mut(usize::from(offset)) else {
            return false;
        };
        *slot = value;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::HighRam;

    #[test]
    fn hram_stops_before_interrupt_enable_register() {
        let mut hram = HighRam::new();
        assert!(hram.write(0xFF80, 0x41));
        assert!(hram.write(0xFFFE, 0x42));
        assert_eq!(hram.read(0xFF80), Some(0x41));
        assert_eq!(hram.read(0xFFFE), Some(0x42));
        assert_eq!(hram.read(0xFF7F), None);
        assert!(!hram.write(0xFFFF, 0xFF));
        assert_eq!(hram.read(0xFFFF), None);
    }
}
