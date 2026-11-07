# Game Boy Color core

This crate is the starting point for an independently developed Game Boy Color emulation core. It is **not a working console emulator yet**.

## Implemented so far

- Parse the cartridge header, including the CGB compatibility flag, cartridge and memory size codes, and header checksum.
- Load and read 32 KiB ROM-only cartridges (type `0x00`).
- Return explicit errors for unsupported cartridge types and inconsistent ROM-only sizes.
- Unit tests using synthetic, self-authored ROM data.

## Not implemented yet

CPU (SM83), memory bus, boot sequence, VRAM/WRAM banking, PPU, APU, timers, interrupts, DMA, joypad, saves, and MBC1/MBC3/MBC5 are **not** implemented. No games can be executed yet. Integration with CorvusSoftware's own API must follow its actual contract when available in this repository.

## Development

From the repository root, with a Rust toolchain installed:

```sh
cargo test -p corvus-gbc
cargo fmt --all -- --check
```

This milestone is based on the [Pan Docs cartridge header reference](https://gbdev.io/pandocs/The_Cartridge_Header.html). No copyrighted game ROMs or proprietary firmware are included.
