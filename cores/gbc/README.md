# Game Boy Color core

This directory contains the independently developed **Game Boy Color core** for CorvusCores, including a path for original Game Boy compatibility. It is **not yet a runnable emulator**.

## Component layout

```text
src/
├── lib.rs              # Crate modules
├── bus/
│   └── mod.rs          # Partial 16-bit address routing
├── cartridge/
│   ├── mod.rs          # Cartridge component exports
│   ├── header.rs       # Header fields, CGB flag, checksum
│   ├── rom_only.rs     # 32 KiB ROM-only cartridge
│   └── tests.rs        # Synthetic cartridge tests
├── memory/
│   ├── mod.rs
│   ├── wram.rs         # Work RAM banking and echo region
│   └── hram.rs         # High RAM
└── video/
    ├── mod.rs
    ├── vram.rs         # VRAM banks, VBK register
    └── oam.rs          # Object attribute memory
```

Components are grouped by responsibility. Future CPU, PPU, audio, DMA, timer, and input components should receive their own modules when there is functioning code for them. Avoid concentrating unrelated hardware in `lib.rs` or the bus.

## Implemented foundations

- Cartridge header parsing: CGB compatibility, cartridge type and RAM/ROM size fields, header checksum.
- 32 KiB ROM-only cartridge loading and reading. Unsupported mapper types and sizes are rejected.
- Partial and **untimed** bus access with explicit unsupported reads/writes.
- 8 banks of WRAM in CGB mode, including `FF70` (SVBK), fixed bank 0, switchable banks 1–7, and the `E000–FDFF` echo.
- 2 VRAM banks in CGB mode, selected through `FF4F` (VBK).
- HRAM at `FF80–FFFE` and OAM storage at `FE00–FE9F`.
- Basic differences between color mode and original Game Boy compatibility mode for the implemented bank selection.
- Tests with synthetic, self-authored cartridge data and boundary/banking cases.

## Not implemented yet

**No games can execute.** There is no SM83 CPU, boot sequence, PPU renderer or timing, APU, timers, interrupt controller, joypad, DMA, save states, or MBC1/MBC3/MBC5. Most I/O registers and access restrictions are intentionally not implemented. A bus read returning `None` means its behavior is not modeled yet, **not** that a real console necessarily returns an absent value.

The current zero-initialized RAM is a deterministic test starting state, not an emulation of real power-on noise. The preliminary mode selection from the cartridge header will eventually belong to boot/hardware initialization. Integration with CorvusSoftware's own API must follow its actual contract when it is available in this repository.

## Development

From the repository root, with the Rust toolchain installed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p corvus-gbc
```

## Hardware references

- [Pan Docs — Memory Map](https://gbdev.io/pandocs/Memory_Map.html)
- [Pan Docs — CGB Registers](https://gbdev.io/pandocs/CGB_Registers.html)
- [Pan Docs — The Cartridge Header](https://gbdev.io/pandocs/The_Cartridge_Header.html)

No copyrighted game ROMs or proprietary firmware are included.
