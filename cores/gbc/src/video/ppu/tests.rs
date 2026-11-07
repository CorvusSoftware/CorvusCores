// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Deterministic PPU register, dot, LY and interrupt regression tests.

use super::Ppu;
use crate::interrupts::{Interrupt, InterruptController};

fn write(ppu: &mut Ppu, irq: &mut InterruptController, address: u16, value: u8) {
    assert!(ppu.write(address, value, irq));
}

fn mode(ppu: &Ppu) -> u8 {
    ppu.read(0xFF41).unwrap() & 0x03
}

#[test]
fn lcd_starts_off_without_clocking_or_requests() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    assert_eq!(ppu.read(0xFF40), Some(0));
    assert_eq!(ppu.read(0xFF41), Some(0x84));
    assert_eq!(ppu.read(0xFF44), Some(0));
    ppu.tick(456 * 154 * 2, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(0));
    assert_eq!(mode(&ppu), 0);
    assert_eq!(irq.read_if(), 0xE0);
}

#[test]
fn visible_line_uses_80_oam_172_transfer_204_hblank_dots() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    assert_eq!(mode(&ppu), 2);
    assert_eq!(ppu.read(0xFF44), Some(0));

    ppu.tick(79, &mut irq);
    assert_eq!(mode(&ppu), 2);
    ppu.tick(1, &mut irq);
    assert_eq!(mode(&ppu), 3);
    ppu.tick(171, &mut irq);
    assert_eq!(mode(&ppu), 3);
    ppu.tick(1, &mut irq);
    assert_eq!(mode(&ppu), 0);
    ppu.tick(203, &mut irq);
    assert_eq!(mode(&ppu), 0);
    ppu.tick(1, &mut irq);
    assert_eq!(mode(&ppu), 2);
    assert_eq!(ppu.read(0xFF44), Some(1));
    assert_eq!(irq.read_if(), 0xE0);
}

#[test]
fn vblank_is_requested_at_line_144_and_frame_wraps_after_153() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF40, 0x80);

    ppu.tick(144 * 456 - 1, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(143));
    assert_eq!(mode(&ppu), 0);
    assert_eq!(irq.read_if(), 0xE0);
    ppu.tick(1, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(144));
    assert_eq!(mode(&ppu), 1);
    assert_eq!(irq.read_if(), 0xE1);

    irq.write_if(0);
    ppu.tick(9 * 456, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(153));
    assert_eq!(mode(&ppu), 1);
    assert_eq!(irq.read_if(), 0xE0);

    ppu.tick(456, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(0));
    assert_eq!(mode(&ppu), 2);
    assert_eq!(irq.read_if(), 0xE0);

    ppu.tick(144 * 456, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(144));
    assert_eq!(irq.read_if(), 0xE1);
}

#[test]
fn stat_mode_sources_request_only_on_a_rising_shared_line() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    write(&mut ppu, &mut irq, 0xFF41, 0x28); // OAM and HBlank sources
    assert_eq!(irq.read_if(), 0xE2);

    irq.write_if(0);
    ppu.tick(80, &mut irq); // mode 2 -> 3, line drops
    assert_eq!(irq.read_if(), 0xE0);
    ppu.tick(172, &mut irq); // mode 3 -> 0, line rises
    assert_eq!(irq.read_if(), 0xE2);

    irq.write_if(0);
    ppu.tick(204, &mut irq); // mode 0 -> 2: still high, no new edge
    assert_eq!(mode(&ppu), 2);
    assert_eq!(irq.read_if(), 0xE0);
    ppu.tick(80 + 172, &mut irq); // mode 2 -> 3 -> 0: new edge
    assert_eq!(irq.read_if(), 0xE2);
}

#[test]
fn stat_mode_zero_and_one_sources_block_each_other_at_vblank_start() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF41, 0x18); // mode 0 and mode 1
    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    ppu.tick(143 * 456 + 252, &mut irq);
    assert_eq!(mode(&ppu), 0);
    assert_eq!(irq.read_if() & Interrupt::LcdStat.mask(), 0x02);

    irq.write_if(0);
    ppu.tick(204, &mut irq); // HBlank -> VBlank, shared line remains high
    assert_eq!(ppu.read(0xFF44), Some(144));
    assert_eq!(mode(&ppu), 1);
    assert_eq!(irq.read_if(), 0xE1); // VBlank yes; no STAT re-trigger
}

#[test]
fn lyc_matches_request_stat_only_on_the_coincidence_edge() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF45, 1);
    write(&mut ppu, &mut irq, 0xFF41, 0x40);
    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    assert_eq!(ppu.read(0xFF41).unwrap() & 0x04, 0);
    assert_eq!(irq.read_if(), 0xE0);

    ppu.tick(456, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(1));
    assert_eq!(ppu.read(0xFF41).unwrap() & 0x04, 0x04);
    assert_eq!(irq.read_if(), 0xE2);
    irq.write_if(0);
    ppu.tick(200, &mut irq);
    assert_eq!(irq.read_if(), 0xE0);

    write(&mut ppu, &mut irq, 0xFF45, 2); // drops shared line
    assert_eq!(irq.read_if(), 0xE0);
    ppu.tick(256, &mut irq); // LY=2: coincidence rises
    assert_eq!(ppu.read(0xFF44), Some(2));
    assert_eq!(irq.read_if(), 0xE2);
}

#[test]
fn stat_writable_mask_and_read_only_ly_are_respected() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF45, 1);
    write(&mut ppu, &mut irq, 0xFF41, 0xFF);
    assert_eq!(ppu.read(0xFF41), Some(0xF8));
    write(&mut ppu, &mut irq, 0xFF44, 100); // LY is read-only
    assert_eq!(ppu.read(0xFF44), Some(0));

    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    assert_eq!(ppu.read(0xFF41), Some(0xFA)); // mode 2
    write(&mut ppu, &mut irq, 0xFF41, 0x03); // status/mode bits ignored
    assert_eq!(ppu.read(0xFF41), Some(0x82));
}

#[test]
fn lcd_disable_resets_scanline_and_freezes_ppu_without_irq() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    ppu.tick(10 * 456 + 100, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(10));
    assert_eq!(mode(&ppu), 3);

    write(&mut ppu, &mut irq, 0xFF40, 0);
    assert_eq!(mode(&ppu), 0);
    assert_eq!(ppu.read(0xFF44), Some(0));
    irq.write_if(0);
    ppu.tick(100_000, &mut irq);
    assert_eq!(ppu.read(0xFF44), Some(0));
    assert_eq!(irq.read_if(), 0xE0);

    write(&mut ppu, &mut irq, 0xFF40, 0x80);
    assert_eq!(ppu.read(0xFF44), Some(0));
    assert_eq!(mode(&ppu), 2);
}

#[test]
fn scroll_palette_window_registers_store_values_without_rendering() {
    let mut ppu = Ppu::new();
    let mut irq = InterruptController::new();
    for (address, value) in [
        (0xFF42, 0xAB),
        (0xFF43, 0x09),
        (0xFF47, 0xE4),
        (0xFF48, 0xD2),
        (0xFF49, 0x18),
        (0xFF4A, 0x80),
        (0xFF4B, 0x01),
    ] {
        write(&mut ppu, &mut irq, address, value);
        assert_eq!(ppu.read(address), Some(value));
    }
    assert_eq!(ppu.read(0xFF46), None); // DMA is not implemented
    assert!(!ppu.write(0xFF46, 0x40, &mut irq));
    assert_eq!(irq.read_if(), 0xE0);
}
