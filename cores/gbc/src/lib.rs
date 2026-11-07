// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Game Boy Color core foundations.
//!
//! Cartridge parsing, memory, partial SM83 execution, timers, interrupts
//! and nominal LCD mode timing have initial implementations. The machine
//! runner clocks peripherals at instruction boundaries, not per bus access.
//! Initial background, window and sprite scanline rendering is available;
//! boot, full pixel timing and CorvusSoftware API integration are absent.
//! OAM DMA and CPU video-memory gating are preliminary instruction-boundary models.

pub mod bus;
pub mod cartridge;
pub mod cpu;
mod dma;
pub mod interrupts;
pub mod machine;
mod memory;
pub mod timer;
mod video;

/// LCD framebuffer dimensions in pixels (row-major RGB555).
pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
