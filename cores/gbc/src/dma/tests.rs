// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

use super::OamDma;

#[test]
fn transfer_outputs_160_pairs_over_exactly_640_t_cycles() {
    let mut dma = OamDma::new();
    assert_eq!(dma.source_high(), 0);
    assert!(!dma.active());

    dma.start(0xC1);
    assert_eq!(dma.source_high(), 0xC1);
    for index in 0_u16..160 {
        for _ in 0..3 {
            assert_eq!(dma.tick_t_cycle(), None);
            assert!(dma.active());
        }
        assert_eq!(dma.tick_t_cycle(), Some((0xC100 + index, 0xFE00 + index)));
    }
    assert!(!dma.active());
    for _ in 0..128 {
        assert_eq!(dma.tick_t_cycle(), None);
    }
}

#[test]
fn new_start_replaces_page_and_resets_partial_progress() {
    let mut dma = OamDma::new();
    dma.start(0xD0);
    for _ in 0..10 {
        dma.tick_t_cycle();
    }
    dma.start(0xC0);
    assert_eq!(dma.source_high(), 0xC0);
    for _ in 0..3 {
        assert_eq!(dma.tick_t_cycle(), None);
    }
    assert_eq!(dma.tick_t_cycle(), Some((0xC000, 0xFE00)));
    assert!(dma.active());
}

#[test]
fn every_source_page_is_latched_without_panicking() {
    let mut dma = OamDma::new();
    for page in [0_u8, 0x7F, 0x9F, 0xDF, 0xE0, 0xFF] {
        dma.start(page);
        for _ in 0..4 {
            dma.tick_t_cycle();
        }
        assert_eq!(dma.source_high(), page);
        assert!(dma.active());
    }
}
