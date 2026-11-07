// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Video-owned VRAM, OAM and the nominal BG/window/OBJ scanline renderer.
//!
//! The bus gates CPU video-memory accesses according to nominal LCD modes;
//! DMA uses independent OAM writes. Fine-grained corruption and fetch timing
//! remain unimplemented.

mod oam;
mod ppu;
mod vram;

pub(crate) use oam::ObjectRam;
pub(crate) use ppu::Ppu;
pub(crate) use vram::VideoRam;
