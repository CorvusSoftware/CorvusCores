// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Self-authored tile patterns and RGB555 framebuffer regression tests.

use super::Ppu;
use crate::interrupts::InterruptController;
use crate::video::{ObjectRam, VideoRam};
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};

fn lcd_write(ppu: &mut Ppu, irq: &mut InterruptController, address: u16, value: u8) {
    assert!(ppu.write(address, value, irq));
}

fn render_line(ppu: &mut Ppu, vram: &VideoRam, irq: &mut InterruptController) {
    ppu.tick(252, vram, &ObjectRam::new(), irq);
}

#[test]
fn dmg_decodes_tile_bitplanes_msb_first_and_maps_bgp_shades() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8000, 0x55));
    assert!(vram.write(0x8001, 0x33));
    lcd_write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);

    ppu.tick(251, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(ppu.framebuffer()[..8], [0x7FFF; 8]);
    ppu.tick(1, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(
        ppu.framebuffer()[..8],
        [0x7FFF, 0x56B5, 0x294A, 0x0000, 0x7FFF, 0x56B5, 0x294A, 0x0000]
    );
    assert_eq!(ppu.framebuffer().len(), SCREEN_WIDTH * SCREEN_HEIGHT);
}

#[test]
fn dmg_bgp_can_remap_raw_tile_colors_without_changing_vram() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8000, 0xFF)); // raw color 1 across the row
    assert!(vram.write(0x9801, 1)); // next map cell is blank tile 1
    lcd_write(&mut ppu, &mut irq, 0xFF47, 0x0C); // color 1 -> shade 3
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);

    render_line(&mut ppu, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x0000);
    assert_eq!(ppu.framebuffer()[7], 0x0000);
    assert_eq!(ppu.framebuffer()[8], 0x7FFF); // next tile is color 0
    assert_eq!(vram.read(0x8000), Some(0xFF));
}

#[test]
fn signed_and_unsigned_tile_data_and_both_background_maps() {
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    assert!(vram.write(0x9800, 0)); // map 0 -> tile 0
    assert!(vram.write(0x9C00, 1)); // map 1 -> tile 1
    assert!(vram.write(0x8000, 0xFF)); // unsigned 0 -> raw color 1
    assert!(vram.write(0x9001, 0xFF)); // signed 0 -> raw color 2
    assert!(vram.write(0x8010, 0xFF)); // unsigned 1 -> raw color 3
    assert!(vram.write(0x8011, 0xFF));

    for (lcdc, expected) in [(0x91, 0x56B5), (0x81, 0x294A), (0x99, 0x0000)] {
        let mut ppu = Ppu::new(false);
        lcd_write(&mut ppu, &mut irq, 0xFF47, 0xE4);
        lcd_write(&mut ppu, &mut irq, 0xFF40, lcdc);
        render_line(&mut ppu, &vram, &ObjectRam::new(), &mut irq);
        assert_eq!(ppu.framebuffer()[0], expected, "LCDC={lcdc:02X}");
    }
}

#[test]
fn scrolling_wraps_at_256_pixels_horizontally_and_vertically() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    for row in 0..8 {
        assert!(vram.write(0x8010 + row * 2, 0xFF)); // tile 1, color 1
        assert!(vram.write(0x8021 + row * 2, 0xFF)); // tile 2, color 2
    }
    assert!(vram.write(0x9BFF, 1)); // map row 31, col 31
    assert!(vram.write(0x9BE0, 2)); // map row 31, col 0
    assert!(vram.write(0x981F, 2)); // map row 0, col 31
    assert!(vram.write(0x9800, 1)); // map row 0, col 0
    lcd_write(&mut ppu, &mut irq, 0xFF42, 248);
    lcd_write(&mut ppu, &mut irq, 0xFF43, 248);
    lcd_write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);

    render_line(&mut ppu, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x56B5);
    assert_eq!(ppu.framebuffer()[8], 0x294A);

    ppu.tick(8 * 456, &vram, &ObjectRam::new(), &mut irq); // visible line 8, scroll Y wraps
    let start = 8 * SCREEN_WIDTH;
    assert_eq!(ppu.framebuffer()[start], 0x294A);
    assert_eq!(ppu.framebuffer()[start + 8], 0x56B5);
}

#[test]
fn framebuffer_is_updated_at_mode_three_end_and_blank_on_lcd_disable() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut irq = InterruptController::new();
    for row in 0..8 {
        assert!(vram.write(0x8000 + row * 2, 0xFF));
    }
    lcd_write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);
    ppu.tick(252, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x56B5);

    for row in 0..8 {
        assert!(vram.write(0x8000 + row * 2, 0));
        assert!(vram.write(0x8001 + row * 2, 0xFF)); // color 2
    }
    ppu.tick(456, &vram, &ObjectRam::new(), &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x56B5); // completed line is immutable
    assert_eq!(ppu.framebuffer()[SCREEN_WIDTH], 0x294A);

    lcd_write(&mut ppu, &mut irq, 0xFF40, 0);
    assert!(ppu.framebuffer().iter().all(|&pixel| pixel == 0x7FFF));
    ppu.tick(456, &vram, &ObjectRam::new(), &mut irq);
    assert!(ppu.framebuffer().iter().all(|&pixel| pixel == 0x7FFF));
}

#[test]
fn dmg_bit_zero_blanks_background_but_cgb_still_draws_it() {
    let mut dmg = Ppu::new(false);
    let mut dmg_vram = VideoRam::new(false);
    let mut cgb = Ppu::new(true);
    let mut cgb_vram = VideoRam::new(true);
    let mut irq = InterruptController::new();
    assert!(dmg_vram.write(0x8000, 0xFF));
    assert!(cgb_vram.write(0x8000, 0xFF));
    lcd_write(&mut dmg, &mut irq, 0xFF47, 0xE4);
    lcd_write(&mut dmg, &mut irq, 0xFF40, 0x90);
    lcd_write(&mut cgb, &mut irq, 0xFF68, 0x82); // palette 0, color 1
    lcd_write(&mut cgb, &mut irq, 0xFF69, 0x1F);
    lcd_write(&mut cgb, &mut irq, 0xFF69, 0x00);
    lcd_write(&mut cgb, &mut irq, 0xFF40, 0x90);

    render_line(&mut dmg, &dmg_vram, &ObjectRam::new(), &mut irq);
    render_line(&mut cgb, &cgb_vram, &ObjectRam::new(), &mut irq);
    assert_eq!(dmg.framebuffer()[0], 0x7FFF);
    assert_eq!(cgb.framebuffer()[0], 0x001F);
}

#[test]
fn cgb_attributes_select_vram_bank_palette_and_horizontal_vertical_flip() {
    let mut ppu = Ppu::new(true);
    let mut vram = VideoRam::new(true);
    let mut irq = InterruptController::new();

    assert!(vram.write_bank_register(1));
    assert!(vram.write(0x9800, 0x6B)); // palette 3, bank 1, X/Y flip
    assert!(vram.write(0x800E, 0x01)); // row 7, rightmost pixel = raw color 1
    assert!(vram.write_bank_register(0));
    assert!(vram.write(0x9800, 0)); // tile zero from bank 0 map
    assert!(vram.write_bank_register(1)); // leave CPU VBK pointing at 1

    lcd_write(&mut ppu, &mut irq, 0xFF68, 0x9A); // palette 3, color 1
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0x1F); // pure red RGB555
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0x00);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);
    render_line(&mut ppu, &vram, &ObjectRam::new(), &mut irq);

    assert_eq!(ppu.framebuffer()[0], 0x001F);
    assert_eq!(ppu.framebuffer()[1], 0x7FFF);
    assert_eq!(ppu.framebuffer()[7], 0x7FFF);
    assert_eq!(ppu.read(0xFF68), Some(0x9C));
}

#[test]
fn cgb_bg_palette_auto_increment_wrap_and_readback() {
    let mut ppu = Ppu::new(true);
    let mut irq = InterruptController::new();
    lcd_write(&mut ppu, &mut irq, 0xFF68, 0xBF);
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0x12);
    assert_eq!(ppu.read(0xFF68), Some(0x80)); // wrapped 0x3F -> 0x00
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0x34);
    assert_eq!(ppu.read(0xFF68), Some(0x81));

    lcd_write(&mut ppu, &mut irq, 0xFF68, 0x3F);
    assert_eq!(ppu.read(0xFF69), Some(0x12));
    assert_eq!(ppu.read(0xFF68), Some(0x3F)); // reading doesn't increment
    lcd_write(&mut ppu, &mut irq, 0xFF68, 0);
    assert_eq!(ppu.read(0xFF69), Some(0x34));
}

#[test]
fn cgb_bgpd_is_blocked_in_mode_three_but_index_still_increments() {
    let mut ppu = Ppu::new(true);
    let vram = VideoRam::new(true);
    let mut irq = InterruptController::new();
    lcd_write(&mut ppu, &mut irq, 0xFF68, 2);
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0x25);
    lcd_write(&mut ppu, &mut irq, 0xFF68, 0x82);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x91);
    ppu.tick(80, &vram, &ObjectRam::new(), &mut irq); // mode 3

    assert_eq!(ppu.read(0xFF69), Some(0xFF));
    lcd_write(&mut ppu, &mut irq, 0xFF69, 0xA7); // ignored write
    assert_eq!(ppu.read(0xFF68), Some(0x83)); // still incremented
    ppu.tick(172, &vram, &ObjectRam::new(), &mut irq); // HBlank
    lcd_write(&mut ppu, &mut irq, 0xFF68, 2);
    assert_eq!(ppu.read(0xFF69), Some(0x25));
}

#[test]
fn cgb_palette_registers_are_unavailable_in_dmg_compatibility() {
    let mut ppu = Ppu::new(false);
    let mut irq = InterruptController::new();
    assert_eq!(ppu.read(0xFF68), None);
    assert_eq!(ppu.read(0xFF69), None);
    assert!(!ppu.write(0xFF68, 0x80, &mut irq));
    assert!(!ppu.write(0xFF69, 0xFF, &mut irq));
}
