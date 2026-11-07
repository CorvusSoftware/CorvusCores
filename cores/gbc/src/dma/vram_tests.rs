// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Register masks, GDMA/HBlank scheduling, cancellation and address wrapping.

use super::vram::VramDma;

fn configure(dma: &mut VramDma, source: u16, destination: u16) {
    assert!(dma.write(0xFF51, (source >> 8) as u8, true));
    assert!(dma.write(0xFF52, source as u8, true));
    assert!(dma.write(0xFF53, (destination >> 8) as u8, true));
    assert!(dma.write(0xFF54, destination as u8, true));
}

#[test]
fn gdma_copies_one_byte_each_two_cycles_and_reports_complete() {
    let mut dma = VramDma::new();
    assert_eq!(dma.read(0xFF55), Some(0xFF));
    configure(&mut dma, 0xC12F, 0x013F); // low nibble masked, dest wraps into VRAM
    assert!(dma.write(0xFF55, 0x00, true));
    assert_eq!(dma.read(0xFF55), Some(0));
    assert!(dma.stalls_cpu());
    for i in 0_u16..16 {
        assert_eq!(dma.tick_t_cycle(), None);
        assert_eq!(dma.tick_t_cycle(), Some((0xC120 + i, 0x8130 + i)));
    }
    assert!(!dma.active());
    assert!(!dma.stalls_cpu());
    assert_eq!(dma.read(0xFF55), Some(0xFF));
}

#[test]
fn hdma_only_transfers_one_block_per_hblank() {
    let mut dma = VramDma::new();
    configure(&mut dma, 0xC000, 0x8000);
    assert!(dma.write(0xFF55, 0x81, true)); // two 16-byte blocks
    assert_eq!(dma.read(0xFF55), Some(0x01));
    assert!(dma.active());
    assert!(!dma.stalls_cpu());
    for _ in 0..500 {
        assert_eq!(dma.tick_t_cycle(), None);
    }
    dma.begin_hblank();
    assert!(dma.stalls_cpu());
    for i in 0_u16..16 {
        assert_eq!(dma.tick_t_cycle(), None);
        assert_eq!(dma.tick_t_cycle(), Some((0xC000 + i, 0x8000 + i)));
    }
    assert!(dma.active());
    assert!(!dma.stalls_cpu());
    assert_eq!(dma.read(0xFF55), Some(0x00));
    dma.begin_hblank();
    for i in 0_u16..16 {
        assert_eq!(dma.tick_t_cycle(), None);
        assert_eq!(dma.tick_t_cycle(), Some((0xC010 + i, 0x8010 + i)));
    }
    assert!(!dma.active());
    assert_eq!(dma.read(0xFF55), Some(0xFF));
}

#[test]
fn hdma_cancel_preserves_remaining_blocks_and_allows_restart() {
    let mut dma = VramDma::new();
    configure(&mut dma, 0xD000, 0x9FE0);
    assert!(dma.write(0xFF55, 0x83, true));
    dma.begin_hblank();
    for _ in 0..32 {
        dma.tick_t_cycle();
    }
    assert_eq!(dma.read(0xFF55), Some(2));
    assert!(dma.write(0xFF55, 0x00, true)); // stop
    assert_eq!(dma.read(0xFF55), Some(0x82));
    assert!(!dma.active());
    dma.begin_hblank();
    assert_eq!(dma.tick_t_cycle(), None);
    assert!(dma.write(0xFF55, 0x00, true)); // new 16-byte GDMA
    assert!(dma.stalls_cpu());
    assert_eq!(dma.read(0xFF55), Some(0x00));
}

#[test]
fn lcd_off_request_uses_gdma_and_lcd_disable_converts_hdma() {
    let mut dma = VramDma::new();
    configure(&mut dma, 0xC000, 0x8000);
    assert!(dma.write(0xFF55, 0x80, false));
    assert!(dma.stalls_cpu());
    for _ in 0..32 {
        dma.tick_t_cycle();
    }
    assert!(!dma.active());

    assert!(dma.write(0xFF55, 0x81, true));
    assert!(dma.active());
    assert!(!dma.stalls_cpu());
    dma.lcd_disabled();
    assert!(dma.stalls_cpu());
}

#[test]
fn invalid_destination_stops_at_end_of_vram() {
    let mut dma = VramDma::new();
    configure(&mut dma, 0xC000, 0x9FF0);
    assert!(dma.write(0xFF55, 0x02, false));
    for _ in 0..32 {
        dma.tick_t_cycle();
    }
    assert!(!dma.active());
    assert_eq!(dma.read(0xFF55), Some(0xFF));
}

#[test]
fn supported_registers_are_write_only_except_ff55() {
    let mut dma = VramDma::new();
    for reg in 0xFF51..=0xFF54 {
        assert_eq!(dma.read(reg), None);
        assert!(dma.write(reg, 0xFF, true));
    }
    assert_eq!(dma.read(0xFF55), Some(0xFF));
    assert_eq!(dma.read(0xFF56), None);
    assert!(!dma.write(0xFF56, 0xFF, true));
}
