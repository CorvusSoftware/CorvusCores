// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Video-owned memory, independent of the PPU's timing and rendering.
//!
//! CPU access restrictions during PPU mode 2/3 and DMA are not modeled yet.

mod oam;
mod ppu;
mod vram;

pub(crate) use oam::ObjectRam;
pub(crate) use ppu::Ppu;
pub(crate) use vram::VideoRam;
