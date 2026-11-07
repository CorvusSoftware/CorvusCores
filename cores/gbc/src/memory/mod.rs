// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Internal work RAM and high RAM components.

mod hram;
mod wram;

pub(crate) use hram::HighRam;
pub(crate) use wram::WorkRam;
