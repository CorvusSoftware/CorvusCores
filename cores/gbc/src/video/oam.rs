//! Object attribute memory (FE00–FE9F).
//!
//! PPU mode restrictions, OAM DMA, and corruption behavior are not yet handled.

pub(crate) struct ObjectRam {
    bytes: [u8; 0xA0],
}

impl ObjectRam {
    pub(crate) fn new() -> Self {
        Self { bytes: [0; 0xA0] }
    }

    pub(crate) fn read(&self, address: u16) -> Option<u8> {
        let offset = address.checked_sub(0xFE00)?;
        self.bytes.get(usize::from(offset)).copied()
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) -> bool {
        let Some(offset) = address.checked_sub(0xFE00) else {
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
    use super::ObjectRam;

    #[test]
    fn maps_160_bytes_without_touching_unusable_area() {
        let mut oam = ObjectRam::new();
        assert!(oam.write(0xFE00, 0x23));
        assert!(oam.write(0xFE9F, 0x45));
        assert_eq!(oam.read(0xFE00), Some(0x23));
        assert_eq!(oam.read(0xFE9F), Some(0x45));
        assert!(!oam.write(0xFEA0, 0x12));
        assert_eq!(oam.read(0xFEA0), None);
        assert_eq!(oam.read(0xFDFF), None);
    }
}
