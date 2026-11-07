// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Window positioning, map selection, independent line counter and CGB attrs.

use super::Ppu;
use crate::interrupts::InterruptController;
use crate::video::{ObjectRam, VideoRam};
use crate::SCREEN_WIDTH;

fn write(ppu: &mut Ppu, irq: &mut InterruptController, address: u16, value: u8) {
    assert!(ppu.write(address, value, irq));
}

fn tick(ppu: &mut Ppu, vram: &VideoRam, irq: &mut InterruptController, dots: u32) {
    ppu.tick(dots, vram, &ObjectRam::new(), irq);
}

#[test]
fn window_overlays_background_at_wx_minus_seven() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8001, 0xFF)); // BG tile 0 color 2
    assert!(vram.write(0x8010, 0xFF)); // window tile 1 color 1
    assert!(vram.write(0x9C00, 1));
    write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    write(&mut ppu, &mut irq, 0xFF4B, 11); // screen X = 4
    write(&mut ppu, &mut irq, 0xFF40, 0xF1);
    tick(&mut ppu, &vram, &mut irq, 252);

    assert_eq!(&ppu.framebuffer()[0..4], &[0x294A; 4]);
    assert_eq!(&ppu.framebuffer()[4..12], &[0x56B5; 8]);
    assert_eq!(ppu.window_line, 1);
}

#[test]
fn wx_zero_clips_first_seven_window_pixels_and_166_shows_one() {
    for (wx, expected_x, drawn) in [(0, 0, true), (166, 159, true), (167, 159, false)] {
        let mut ppu = Ppu::new(false);
        let mut vram = VideoRam::new(false);
        let mut irq = InterruptController::new();
        assert!(vram.write(0x8010, 0x01)); // window tile 1: only pixel 7
        assert!(vram.write(0x9C00, 1));
        write(&mut ppu, &mut irq, 0xFF47, 0xE4);
        write(&mut ppu, &mut irq, 0xFF4B, wx);
        write(&mut ppu, &mut irq, 0xFF40, 0xF1);
        tick(&mut ppu, &vram, &mut irq, 252);
        assert_eq!(ppu.window_line, u8::from(drawn), "WX={wx}");
        // WX=0 -> screen 0 corresponds to window pixel 7.
        // WX=166 -> screen 159 corresponds to window pixel 0 (transparent zero).
        if wx == 0 {
            assert_eq!(ppu.framebuffer()[expected_x], 0x56B5);
            assert_eq!(ppu.framebuffer()[1], 0x7FFF);
        } else {
            assert_eq!(ppu.framebuffer()[expected_x], 0x7FFF);
        }
    }
}

#[test]
fn window_line_counter_advances_only_when_visible() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0xFF)); // row 0, color 1
    assert!(vram.write(0x8013, 0xFF)); // row 1, color 2
    assert!(vram.write(0x9C00, 1));
    write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    write(&mut ppu, &mut irq, 0xFF4B, 167); // off screen
    write(&mut ppu, &mut irq, 0xFF40, 0xF1);
    tick(&mut ppu, &vram, &mut irq, 2 * 456);
    assert_eq!(ppu.window_line, 0);

    write(&mut ppu, &mut irq, 0xFF4B, 7);
    tick(&mut ppu, &vram, &mut irq, 252);
    assert_eq!(ppu.framebuffer()[2 * SCREEN_WIDTH], 0x56B5);
    assert_eq!(ppu.window_line, 1);

    tick(&mut ppu, &vram, &mut irq, 456);
    assert_eq!(ppu.framebuffer()[3 * SCREEN_WIDTH], 0x294A);
    assert_eq!(ppu.window_line, 2);
}

#[test]
fn wy_latch_starts_at_matching_scanline_and_survives_dmg_wy_rewrite() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0xFF));
    assert!(vram.write(0x9C00, 1));
    write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    write(&mut ppu, &mut irq, 0xFF4A, 2);
    write(&mut ppu, &mut irq, 0xFF4B, 7);
    write(&mut ppu, &mut irq, 0xFF40, 0xF1);

    tick(&mut ppu, &vram, &mut irq, 456 + 252);
    assert_eq!(ppu.framebuffer()[0], 0x7FFF);
    assert_eq!(ppu.framebuffer()[SCREEN_WIDTH], 0x7FFF);
    assert_eq!(ppu.window_line, 0);

    tick(&mut ppu, &vram, &mut irq, 204 + 252); // LY 2
    assert_eq!(ppu.framebuffer()[2 * SCREEN_WIDTH], 0x56B5);
    assert_eq!(ppu.window_line, 1);

    write(&mut ppu, &mut irq, 0xFF4A, 100);
    tick(&mut ppu, &vram, &mut irq, 456);
    assert_eq!(ppu.window_line, 2); // WY condition was latched
}

#[test]
fn cgb_window_uses_bank_one_attributes_and_works_with_lcdc_zero() {
    let mut ppu = Ppu::new(true);
    let mut vram = VideoRam::new(true);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x9C00, 1));
    assert!(vram.write_bank_register(1));
    assert!(vram.write(0x9C00, 0x6A)); // palette 2, bank 1, XY flip
    assert!(vram.write(0x801E, 0x01)); // tile 1, row 7, rightmost pixel
    assert!(vram.write_bank_register(0));
    write(&mut ppu, &mut irq, 0xFF68, 0x92); // BG palette 2, color 1
    write(&mut ppu, &mut irq, 0xFF69, 0x00);
    write(&mut ppu, &mut irq, 0xFF69, 0x7C); // RGB555 blue
    write(&mut ppu, &mut irq, 0xFF4B, 7);
    write(&mut ppu, &mut irq, 0xFF40, 0xF0); // native CGB: LCDC.0 does not blank BG/Window
    tick(&mut ppu, &vram, &mut irq, 252);

    assert_eq!(ppu.framebuffer()[0], 0x7C00);
    assert_eq!(ppu.framebuffer()[1], 0x7FFF);
}

#[test]
fn window_resets_to_first_row_after_new_frame() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0xFF));
    assert!(vram.write(0x9C00, 1));
    write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    write(&mut ppu, &mut irq, 0xFF4B, 7);
    write(&mut ppu, &mut irq, 0xFF40, 0xF1);
    tick(&mut ppu, &vram, &mut irq, 3 * 456);
    assert_eq!(ppu.window_line, 3);

    tick(&mut ppu, &vram, &mut irq, 151 * 456); // total 154 lines: new frame
    assert_eq!(ppu.window_line, 0);
    tick(&mut ppu, &vram, &mut irq, 252);
    assert_eq!(ppu.window_line, 1);
    assert_eq!(ppu.framebuffer()[0], 0x56B5);
}
