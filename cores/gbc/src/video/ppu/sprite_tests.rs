// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Object selection, transparency, DMG/CGB priority, flips and palettes.

use super::Ppu;
use crate::interrupts::InterruptController;
use crate::video::{ObjectRam, VideoRam};

fn lcd_write(ppu: &mut Ppu, irq: &mut InterruptController, address: u16, value: u8) {
    assert!(ppu.write(address, value, irq));
}

fn object(oam: &mut ObjectRam, slot: u16, y: u8, x: u8, tile: u8, attr: u8) {
    let start = 0xFE00 + slot * 4;
    for (offset, value) in [y, x, tile, attr].into_iter().enumerate() {
        assert!(oam.write(start + offset as u16, value));
    }
}

fn render(ppu: &mut Ppu, vram: &VideoRam, oam: &ObjectRam, irq: &mut InterruptController) {
    ppu.tick(252, vram, oam, irq);
}

#[test]
fn dmg_sprite_renders_over_transparent_bg_with_color_zero_transparent() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0x80)); // OBJ tile 1: only first pixel color 1
    object(&mut oam, 0, 16, 8, 1, 0);
    lcd_write(&mut ppu, &mut irq, 0xFF48, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);

    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x56B5);
    assert_eq!(ppu.framebuffer()[1], 0x7FFF);
    assert_eq!(ppu.framebuffer()[8], 0x7FFF);
}

#[test]
fn object_pixels_fall_through_transparent_front_sprite() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0x80)); // OBJ0: only screen x=0
    assert!(vram.write(0x8021, 0xFF)); // OBJ1: all pixels are color 2
    object(&mut oam, 0, 16, 8, 1, 0);
    object(&mut oam, 1, 16, 8, 2, 0);
    lcd_write(&mut ppu, &mut irq, 0xFF48, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);

    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x56B5);
    assert_eq!(ppu.framebuffer()[1], 0x294A);
}

#[test]
fn dmg_prefers_smaller_x_but_cgb_prefers_earlier_oam() {
    let mut dmg = Ppu::new(false);
    let mut cgb = Ppu::new(true);
    let mut vram = VideoRam::new(true);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0xFF)); // color 1
    assert!(vram.write(0x8021, 0xFF)); // color 2
    object(&mut oam, 0, 16, 12, 1, 0); // earlier entry, screen X 4
    object(&mut oam, 1, 16, 10, 2, 0); // smaller X, screen X 2

    lcd_write(&mut dmg, &mut irq, 0xFF48, 0xE4);
    lcd_write(&mut dmg, &mut irq, 0xFF40, 0x93);
    lcd_write(&mut cgb, &mut irq, 0xFF6A, 0x82);
    lcd_write(&mut cgb, &mut irq, 0xFF6B, 0x1F); // OBJ palette 0, color 1 red
    lcd_write(&mut cgb, &mut irq, 0xFF6B, 0);
    lcd_write(&mut cgb, &mut irq, 0xFF6B, 0xE0); // OBJ palette 0, color 2 green
    lcd_write(&mut cgb, &mut irq, 0xFF6B, 3);
    lcd_write(&mut cgb, &mut irq, 0xFF40, 0x93);

    render(&mut dmg, &vram, &oam, &mut irq);
    render(&mut cgb, &vram, &oam, &mut irq);
    assert_eq!(dmg.framebuffer()[4], 0x294A);
    assert_eq!(cgb.framebuffer()[4], 0x001F);
}

#[test]
fn only_first_ten_y_overlapping_oam_entries_are_selected() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0xFF));
    for slot in 0..10 {
        object(&mut oam, slot, 16, 0, 1, 0); // invisible X, but selected
    }
    object(&mut oam, 10, 16, 8, 1, 0); // visible but not selected
    lcd_write(&mut ppu, &mut irq, 0xFF48, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);
    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x7FFF);
}

#[test]
fn obj_8x16_uses_even_tile_number_and_vertical_flip() {
    for (attributes, expected) in [(0, 0x56B5), (0x40, 0x294A)] {
        let mut ppu = Ppu::new(false);
        let mut vram = VideoRam::new(false);
        let mut oam = ObjectRam::new();
        let mut irq = InterruptController::new();
        assert!(vram.write(0x8020, 0xFF)); // even tile 2, first row color 1
        assert!(vram.write(0x803F, 0xFF)); // tile 3 last row color 2
        object(&mut oam, 0, 16, 8, 3, attributes); // odd ID must become even
        lcd_write(&mut ppu, &mut irq, 0xFF48, 0xE4);
        lcd_write(&mut ppu, &mut irq, 0xFF40, 0x97); // 8x16 OBJ
        render(&mut ppu, &vram, &oam, &mut irq);
        assert_eq!(ppu.framebuffer()[0], expected);
    }
}

#[test]
fn dmg_obj_second_palette_and_horizontal_flip() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8010, 0x01)); // rightmost pixel is color 1
    object(&mut oam, 0, 16, 8, 1, 0x30); // palette 1, X-flip
    lcd_write(&mut ppu, &mut irq, 0xFF49, 0x0C); // color 1 -> black
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);
    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0);
    assert_eq!(ppu.framebuffer()[1], 0x7FFF);
}

#[test]
fn higher_priority_obj_hidden_by_bg_still_masks_lower_obj() {
    let mut ppu = Ppu::new(false);
    let mut vram = VideoRam::new(false);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write(0x8001, 0xFF)); // BG tile 0 color 2
    assert!(vram.write(0x8010, 0xFF)); // OBJ tile 1 color 1
    assert!(vram.write(0x8020, 0xFF));
    assert!(vram.write(0x8021, 0xFF)); // OBJ tile 2 color 3
    object(&mut oam, 0, 16, 8, 1, 0x80); // behind nonzero BG
    object(&mut oam, 1, 16, 8, 2, 0);
    lcd_write(&mut ppu, &mut irq, 0xFF47, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF48, 0xE4);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);
    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x294A); // BG, not lower OBJ
}

#[test]
fn cgb_bg_priority_and_lcdc_master_priority_select_winner() {
    for (lcdc, expected) in [(0x93, 0x001F), (0x92, 0x03E0)] {
        let mut ppu = Ppu::new(true);
        let mut vram = VideoRam::new(true);
        let mut oam = ObjectRam::new();
        let mut irq = InterruptController::new();
        assert!(vram.write(0x8000, 0xFF)); // BG raw color 1
        assert!(vram.write(0x8011, 0xFF)); // OBJ raw color 2
        assert!(vram.write_bank_register(1));
        assert!(vram.write(0x9800, 0x80)); // BG priority
        assert!(vram.write_bank_register(0));
        object(&mut oam, 0, 16, 8, 1, 0);
        lcd_write(&mut ppu, &mut irq, 0xFF68, 0x82);
        lcd_write(&mut ppu, &mut irq, 0xFF69, 0x1F);
        lcd_write(&mut ppu, &mut irq, 0xFF69, 0);
        lcd_write(&mut ppu, &mut irq, 0xFF6A, 0x84);
        lcd_write(&mut ppu, &mut irq, 0xFF6B, 0xE0);
        lcd_write(&mut ppu, &mut irq, 0xFF6B, 0x03);
        lcd_write(&mut ppu, &mut irq, 0xFF40, lcdc);
        render(&mut ppu, &vram, &oam, &mut irq);
        assert_eq!(ppu.framebuffer()[0], expected);
    }
}

#[test]
fn cgb_obj_uses_bank_one_flip_and_palette_selection() {
    let mut ppu = Ppu::new(true);
    let mut vram = VideoRam::new(true);
    let mut oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    assert!(vram.write_bank_register(1));
    assert!(vram.write(0x8010, 0x01)); // OBJ bank1 tile1 rightmost pixel
    assert!(vram.write_bank_register(0));
    object(&mut oam, 0, 16, 8, 1, 0x29); // palette 1, bank1, X flip
    lcd_write(&mut ppu, &mut irq, 0xFF6A, 0x8A); // palette1 color1
    lcd_write(&mut ppu, &mut irq, 0xFF6B, 0);
    lcd_write(&mut ppu, &mut irq, 0xFF6B, 0x7C); // RGB555 blue
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);
    render(&mut ppu, &vram, &oam, &mut irq);
    assert_eq!(ppu.framebuffer()[0], 0x7C00);
    assert_eq!(ppu.framebuffer()[1], 0x7FFF);
}

#[test]
fn object_palette_index_wraps_and_data_is_blocked_in_mode_three() {
    let mut ppu = Ppu::new(true);
    let vram = VideoRam::new(true);
    let oam = ObjectRam::new();
    let mut irq = InterruptController::new();
    lcd_write(&mut ppu, &mut irq, 0xFF6A, 0xBF);
    lcd_write(&mut ppu, &mut irq, 0xFF6B, 0x55);
    assert_eq!(ppu.read(0xFF6A), Some(0x80));
    lcd_write(&mut ppu, &mut irq, 0xFF6A, 0x82);
    lcd_write(&mut ppu, &mut irq, 0xFF6B, 0x25);
    lcd_write(&mut ppu, &mut irq, 0xFF6A, 0x82);
    lcd_write(&mut ppu, &mut irq, 0xFF40, 0x93);
    ppu.tick(80, &vram, &oam, &mut irq);
    assert_eq!(ppu.read(0xFF6B), Some(0xFF));
    lcd_write(&mut ppu, &mut irq, 0xFF6B, 0xA7); // ignored but increments
    assert_eq!(ppu.read(0xFF6A), Some(0x83));
    ppu.tick(172, &vram, &oam, &mut irq);
    lcd_write(&mut ppu, &mut irq, 0xFF6A, 2);
    assert_eq!(ppu.read(0xFF6B), Some(0x25));

    let mut dmg = Ppu::new(false);
    assert_eq!(dmg.read(0xFF6A), None);
    assert_eq!(dmg.read(0xFF6B), None);
    assert!(!dmg.write(0xFF6A, 0, &mut irq));
    assert!(!dmg.write(0xFF6B, 0, &mut irq));
}
