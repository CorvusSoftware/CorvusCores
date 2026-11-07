//! Video-owned memory, independent of the PPU's timing and rendering.
//!
//! CPU access restrictions during PPU mode 2/3 and DMA are not modeled yet.

mod oam;
mod vram;

pub(crate) use oam::ObjectRam;
pub(crate) use vram::VideoRam;
