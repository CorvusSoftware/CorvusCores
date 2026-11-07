// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Video-owned VRAM, OAM and the nominal PPU background renderer.
//!
//! CPU VRAM/OAM access restrictions during PPU modes and DMA are not modeled.

mod oam;
mod ppu;
mod vram;

pub(crate) use oam::ObjectRam;
pub(crate) use ppu::Ppu;
pub(crate) use vram::VideoRam;
