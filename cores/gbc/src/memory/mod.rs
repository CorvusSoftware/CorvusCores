//! Internal work RAM and high RAM components.

mod hram;
mod wram;

pub(crate) use hram::HighRam;
pub(crate) use wram::WorkRam;
