# Game Boy Color core

This directory contains the independently developed **Game Boy Color core** for CorvusCores, including a path for original Game Boy compatibility. 

**A basic SDL2 frontend is now available** via the `corvus_gbc` binary, allowing you to run ROMs and see the display.

## Component layout

```text
src/
├── lib.rs              # Crate modules
├── bus/
│   ├── mod.rs          # Bus ownership, hardware mode and interrupt facade
│   ├── cpu_access.rs   # CPU memory routing, MBC writes and LCD gating
│   ├── clock.rs        # Peripheral T-cycle order and mapped DMA sources
│   ├── cartridge_tests.rs # MBC1 CPU, Machine and DMA integration tests
│   ├── mbc3_tests.rs  # MBC3 mapper/RTC through CPU, DMA and Machine
│   ├── tests.rs        # ROM, banking and unmapped-I/O routing contracts
│   ├── dma_tests.rs    # OAM DMA transfers and CPU access restrictions
│   ├── vram_dma_tests.rs # CGB GDMA/HDMA bus and HBlank tests
│   └── timing_tests.rs # Timer/interrupt bus integration tests
├── cpu/
│   ├── mod.rs          # CPU state and public exports
│   ├── registers.rs    # SM83 registers and F flag invariants
│   ├── execute.rs      # Base opcode dispatch and nominal T-cycle results
│   ├── control_flow.rs # JR/JP, CALL/RET, RST and condition evaluation
│   ├── control_flow_tests.rs # Conditional branches, cycles and error contracts
│   ├── interrupts.rs   # IME, EI/DI/RETI, IRQ service and HALT behavior
│   ├── interrupt_tests.rs # Delays, IRQ priority and HALT regression tests
│   ├── load.rs         # Indirect, indexed, and high-memory transfers
│   ├── load_tests.rs   # Transfer, addressing, and bus-error tests
│   ├── cb.rs           # All 256 CB-prefixed rotate/shift/bit opcodes
│   ├── cb_tests.rs     # CB behavior and timing regression tests
│   ├── stack.rs        # Stack operations and register-pair transfers
│   ├── stack_tests.rs  # PUSH/POP/CALL/RET/RST regression tests
│   ├── alu_tests.rs    # Arithmetic, flags and BCD regression tests
│   ├── alu.rs          # Arithmetic and flag calculations
│   ├── error.rs        # Unsupported opcodes and unmapped bus errors
│   └── tests.rs        # Self-authored CPU instruction fixtures
├── cartridge/
│   ├── mod.rs          # Public cartridge/mapper exports
│   ├── mapper.rs       # Type-safe ROM-only/MBC1/MBC3 mapper delegation
│   ├── mbc1.rs         # ROM/RAM bank selection and MBC1 control latches
│   ├── mbc1_tests.rs   # Synthetic ROM/RAM bank regression tests
│   ├── mbc3.rs         # MBC3 ROM/RAM banks, RTC selection and latch
│   ├── mbc3_tests.rs   # MBC3 mapper and external SRAM regression tests
│   ├── rtc.rs          # Independent MBC3 live/latched RTC counters
│   ├── rtc_tests.rs    # Seconds, halt, day carry and latch tests
│   ├── error.rs        # Cartridge errors and image length validation
│   ├── header.rs       # Header fields, CGB flag, checksum
│   ├── rom_only.rs     # Legacy-compatible 32 KiB ROM-only cartridge
│   └── tests.rs        # Header and ROM-only cartridge tests
├── dma/
│   ├── mod.rs          # OAM DMA transfer clock and source page latch
│   ├── tests.rs        # OAM DMA cadence and restart tests
│   ├── vram.rs         # CGB GDMA/HDMA source, destination and block engine
│   └── vram_tests.rs   # CGB VRAM DMA register/timing tests
├── memory/
│   ├── mod.rs
│   ├── wram.rs         # Work RAM banking and echo region
│   └── hram.rs         # High RAM
├── machine/
│   ├── mod.rs          # CPU/bus ownership and nominal cycle counters
│   ├── scheduler.rs    # CPU vs. VRAM DMA step scheduling and bus ticking
│   ├── scheduler_tests.rs # CPU/DMA cycle accounting regression tests
│   └── tests.rs        # HALT, timer, LCD and DMA integration tests
├── interrupts/
│   ├── mod.rs          # IF/IE, request flags, priority and vectors
│   └── tests.rs        # Register and priority regression tests
├── timer/
│   ├── mod.rs          # DIV/TIMA/TMA/TAC and explicit T-cycle clock
│   └── tests.rs        # Clock sources and overflow timing tests
└── video/
    ├── mod.rs
    ├── vram.rs         # VRAM banks, VBK register
    ├── oam.rs          # Object attribute memory
    └── ppu/
        ├── mod.rs       # LCD state and row-major RGB555 framebuffer
        ├── registers.rs # LCDC/STAT/LY/LYC, scroll and palette registers
        ├── timing.rs    # Nominal scanline, VBlank and STAT edge timing
        ├── background.rs  # Shared BG/window tile sampling, raw color metadata
        ├── window.rs      # Window position, map selection and visible-line counter
        ├── sprites.rs     # OBJ selection, transparency and priority compositing
        ├── palette.rs     # CGB BG/OBJ palette RAM and RGB555 lookup
        ├── render_tests.rs # BG tile, scrolling and palette pixel tests
        ├── window_tests.rs # WX/WY, internal window line counter and CGB attrs
        ├── sprite_tests.rs # 8x8/8x16 OBJ, priority, flips and palette tests
        └── tests.rs      # LCD mode and interrupt regression tests
```

Components are grouped by responsibility. Future audio and input components should receive their own modules when there is functioning code for them. Avoid concentrating unrelated hardware in `lib.rs` or the bus.

The bus maintains a single owner for cartridge, memories, timers, PPU and both DMA controllers. Its CPU-visible `read`/`write` routing lives in `bus/cpu_access.rs`; peripheral scheduling and distinct internal DMA source paths live in `bus/clock.rs`. Cartridge banking is owned by the `cartridge/` component: a type-safe `Cartridge` delegates ROM/RAM reads and writes to ROM-only, MBC1 or MBC3. Both `Bus::new` and `Machine::new` accept existing `RomOnlyCartridge` values as well as a loaded `Cartridge`, without introducing mapper details into the bus.

## Implemented foundations

- Cartridge header parsing: CGB compatibility, cartridge type and RAM/ROM size fields, header checksum.
- `Cartridge::load(Vec<u8>)` selects ROM-only (type `00`), MBC1 (types `01/02/03`) or MBC3 (types `0F–13`) from the parsed header; unsupported types (including MBC2/MBC5) are rejected rather than misinterpreted. The original `RomOnlyCartridge::new` remains available, with its exact 32 KiB/no-RAM validation.
- MBC1 standard banking: 2–128 ROM banks (32 KiB–2 MiB, size codes `00–06`), bank 1 initially mapped at `4000–7FFF`, low-bank register zero remapped to 1, upper ROM bits masked to connected bank address lines, advanced mode optionally maps banks `20/40/60` at `0000–3FFF`. RAM enable via low nibble `0A`, volatile 8 KiB or 32 KiB RAM for types `02/03`, four RAM banks in advanced mode, and disabled/absent RAM returning `FF`. MBC1M multicart wiring, 2 KiB RAM and battery-backed persistence are not implemented. The declared ROM byte length is checked before loading.
- **Standard MBC3 (types `0F–13`)**: fixed 16 KiB ROM bank 0 plus switchable bank 1–127 with zero mapped to one, 32 KiB–2 MiB ROM validation, volatile 8/32 KiB SRAM for RAM-bearing types, and RAM/RTC enable via low nibble `0A`. Selecting `00–03` at `4000–5FFF` maps RAM banks; `08–0C` accesses RTC seconds/minutes/hours/day-low/day-high registers on *timer-bearing types `0F/10` only*. The RTC has a live clock, a frozen read snapshot captured by writing `00` then `01` to `6000–7FFF`, a halt bit, and a sticky carry on 512-day wrap. The high-day control register masks reserved bits. `Cartridge::advance_rtc_seconds(u64)`, `Bus::advance_rtc_seconds(u64)` and `Machine::advance_rtc_seconds(u64)` supply elapsed independent seconds explicitly, and never infer RTC progress from CPU T-cycles. This ensures deterministic tests and leaves future host-clock integration separate. Native MBC30 64 KiB RAM, save persistence, RTC host-time sync and exotic RTC register write timing are not implemented.
- Partial bus access with explicit unsupported reads/writes and manual T-cycle advance through `Bus::tick_t_cycles`. Standalone `Cpu::step` deliberately does not tick peripherals. CPU VRAM reads return FF and writes are ignored in LCD mode 3; CPU OAM access is blocked in modes 2/3 and while OAM DMA is active, without blocking internal PPU/DMA memory accesses.
- An instruction-boundary `Machine` runner owns a CPU and bus. `machine/scheduler.rs` selects a CPU step or a GDMA/HDMA stall before ticking the timer, OAM DMA, CGB VRAM DMA and nominal LCD mode clock. DMA stalls consume four-T-cycle quanta without fetching an opcode; CPU steps include HALT idle and interrupt entry. `Machine::step()` still returns the nominal T-cycles consumed. `elapsed_t_cycles()` is the total, `cpu_t_cycles()` counts CPU steps (including HALT and IRQ entry), and `dma_stall_t_cycles()` counts VRAM DMA pauses; **total = CPU + DMA stalls**. A timer or LCD interrupt generated during one step is observed on the next. This is **not** per-M-cycle/per-memory-access synchronization.
- Timer registers `FF04–FF07`: 16-bit divider, all four TIMA clock selections, falling-edge increments, DIV reset, active TAC source switching, 4-T-cycle delayed TIMA reload and interrupt request. TIMA writes during a pending reload cancel that reload.
- Interrupt registers IF (`FF0F`) and IE (`FFFF`): software and device requests, five priority levels, vector lookup, mask filtering, and acknowledgement.
- SM83 interrupt handling: private CPU IME flag, delayed EI, immediate DI and RETI, priority vector service (20 nominal T-cycles), IF acknowledgement, return-address stack writes, HALT sleep/wake, and the pending-interrupt HALT bug. HALT idle steps and interrupt entry do not automatically tick the bus.
- Partial SM83 CPU interpreter: register pairs and flag masking, all eight 8-bit accumulator ALU families (ADD, ADC, SUB, SBC, AND, XOR, OR, CP), including register, (HL), and immediate forms; JP/JR branches, and nominal instruction T-cycle counts.
- SM83 transfer module: LD (BC/DE),A and LD A,(BC/DE), auto-increment/decrement LD (HL+/-),A and LD A,(HL+/-), high-memory LDH (a8),A / LDH A,(a8) and LD (C),A / LD A,(C), plus absolute LD (a16),A / LD A,(a16). Transfers preserve flags, use wrapping HL arithmetic, and fail explicitly for unsupported I/O rather than fabricating register behavior.
- Additional arithmetic/flag instructions: DAA, CPL, SCF, CCF, RLCA/RRCA/RLA/RRA, ADD HL,r16, ADD SP,e8, and LD HL,SP+e8. Unit tests exercise carry/half-carry edge cases and two-digit BCD addition/subtraction.
- SM83 stack and control flow: PUSH/POP (including AF masking), CALL/RET (conditional and unconditional), RST vectors, JP HL, LD SP,HL and LD (a16),SP. Stack accesses use the current partial bus and report unmapped-address errors. The condition decoder and JR/JP/CALL/RET/RST execution now live in `cpu/control_flow.rs`; `cpu/execute.rs` remains the opcode dispatcher, and `cpu/stack.rs` owns stack access semantics. This extraction preserves PC/SP side effects, flags, conditional cycle counts and unsupported-opcode errors.
- The complete 256-entry **CB-prefixed** SM83 instruction space: RLC, RRC, RL, RR, SLA, SRA, SWAP, SRL, BIT, RES and SET, for registers and `(HL)` where applicable. BIT preserves carry and does not write its operand; `(HL)` variants have distinct nominal timings.
- Explicit CPU errors for unsupported opcodes and unmapped hardware, rather than silently returning success.
- 8 banks of WRAM in CGB mode, including `FF70` (SVBK), fixed bank 0, switchable banks 1–7, and the `E000–FDFF` echo.
- 2 VRAM banks in CGB mode, selected through `FF4F` (VBK). The PPU accesses each bank independently of CPU-selected VBK: bank 0 holds background tile IDs, bank 1 holds CGB attributes and optionally tile patterns.
- **160×144 RGB555 framebuffer** available as read-only `Bus::framebuffer()` (also via `Machine::bus().framebuffer()`). Supports background scrolling and wrapping, both tile maps, signed/unsigned tile-pattern addressing, BGP shades in compatibility mode, and native CGB tile-bank, palette and X/Y flip attributes.
- **Window overlay:** WX minus 7 and WY-based vertical triggering, independent window map selection, BG/window shared tile sampling, CGB attributes and a dedicated window-line counter that advances only on visible window lines. WX=0 clipping and WX=166/167 visibility boundaries are covered by synthetic tests. Subtle mid-scanline WY/WX/LCDC interactions are not represented.
- **Sprite (OBJ) scanline rendering:** 8×8 and 8×16 objects, 2bpp colors with color 0 transparent, flipping, DMG OBP0/OBP1, CGB palette and VRAM-bank selection, ten OAM entries selected per scanline, DMG smaller-X vs. CGB earlier-OAM priority, BG/OBJ priority including CGB LCDC.0 master override. OAM scan occurs at line render time, not at its real dot positions.
- Native CGB BG/OBJ palette RAM via BGPI/BGPD (`FF68/FF69`) and OBPI/OBPD (`FF6A/FF6B`): separate 64-byte arrays, auto-increment and wrap, and blocked palette data access during mode 3 (writes still increment their index). The 15-bit colors are exported directly as RGB555 without LCD color correction.
- First PPU milestone: LCDC/STAT, LY/LYC and scroll/window/DMG palette register storage (FF40–FF45 and FF47–FF4B); nominal mode 2/3/0 progression across 456-dot visible scanlines, VBlank on LY=144–153, VBlank IRQ on line 144 and edge-triggered shared STAT IRQ. LCD disable resets and freezes the line clock.
- **FF46 OAM DMA:** copy 160 bytes from a source page to FE00–FE9F over 640 normal-speed T-cycles (one byte per four T-cycles). Supports mapper-selected ROM, mapped external RAM (or `FF` when absent/disabled), VRAM, and WRAM as source; a fresh FF46 write restarts the transfer. Readback returns the latched page; `Bus::oam_dma_active()` exposes ongoing status. Unsupported source pages E0–FF currently yield a deterministic FF placeholder rather than actual bus-dependent values.
- **CGB VRAM DMA FF51–FF55:** masked 16-byte-aligned source and VRAM destination addresses; source mapper-selected ROM/external RAM/WRAM (absent or disabled RAM yields FF), destination uses current VRAM bank `VBK`; FF51–FF54 are write-only, FF55 reports remaining blocks or FF when complete. GDMA transfers 16–2048 bytes continuously and stalls CPU; HDMA transfers one 16-byte block at the start of each visible HBlank, pausing CPU for 32 normal-speed T-cycles and skipping VBlank. Clearing FF55 bit 7 stops an active HDMA with remaining-block status. Starting HDMA with LCD off uses GDMA; switching LCD off converts pending HDMA to continuous transfer. Bus queries: `vram_dma_active()` and `vram_dma_cpu_stalled()`.
- **Simplified LCD timings:** mode 2 lasts 80 dots, mode 3 always 172 dots, and mode 0 uses the remaining 204 dots. The entire BG/window/OBJ scanline is sampled when mode 3 ends (dot 252), rather than pixel by pixel. Real mode 3 varies with scrolling, window and sprite fetch stalls; the LY=153 early rollover, LCD startup, mid-line register writes, actual mode-2 OAM scanning, *per-access timing* of VRAM/OAM CPU blocking and CGB double-speed conversion remain unimplemented.
- HRAM at `FF80–FFFE` and OAM storage at `FE00–FE9F`.
- Basic differences between color mode and original Game Boy compatibility mode for the implemented bank selection.
- Tests with synthetic, self-authored cartridge data and boundary/banking cases.

## Not implemented yet

**No games can execute end to end.** STOP and CGB speed-switching, boot behavior, fine-grained interrupt/bus timing and cycle-accurate scheduling are not yet implemented. The CGB-specific edge cases for TAC writes and same-cycle timer reload writes still require hardware-reference verification. The PPU renders BG/window/OBJ scanlines but is **not a cycle-accurate pixel fetcher** and has no frontend display. APU, joypad, save states and MBC5 are absent. MBC3 is present with an explicitly clocked RTC, not with host-wall-clock synchronization or battery-backed persistence; MBC1 battery saves and MBC1M multicarts also remain unimplemented. Most I/O registers and access restrictions are intentionally not implemented. A bus read returning `None` means its behavior is not modeled yet, **not** that a real console necessarily returns an absent value.

The current zero-initialized RAM, CPU and RTC registers are deterministic test starting states, not simulated hardware power-on conditions. The RTC runs **only** when external code calls `advance_rtc_seconds` with a duration; normal `Machine::step`, `Bus::tick_t_cycles` and CPU HALT do not imply wall-clock RTC advancement. Latched reads remain frozen between explicit `00→01` snapshots; writes target live counters. Battery/offline progression must be implemented by a future host API. CPU cycle counts (including HALT idle steps and interrupt service) are nominal timings. `Machine::step` advances peripherals **after each successful CPU or VRAM-DMA stall step**; direct `Cpu::step` calls do not tick the bus. Manual calls to `Bus::tick_t_cycles` are not included in any `Machine` counter. On CPU errors, no cycle counters or peripherals advance, although a partially completed CPU access can have side effects. Timer and LCD register writes inside an instruction therefore take effect before its entire nominal cycle count is advanced, which is not real hardware timing. The framebuffer is composited line by line at the end of nominal mode 3 and reset to white on LCD disable. OAM DMA copies use a fixed four-T-cycle cadence, but exact startup delay, CPU/DMA bus conflicts (different on DMG/CGB), mid-instruction access ordering, collisions with the pixel fetcher, and CGB double-speed behavior are **not** simulated. FF46 is handled without a complete DMA blocking model for CPU ROM/WRAM accesses. VRAM GDMA/HDMA use one byte per two normal-speed T-cycles and pause opcode execution, but use instruction-level entry points, not exact bus arbitration. Unsupported VRAM-DMA source regions (including VRAM itself) currently read deterministic FF; real bus conflict values, GDMA during active pixel transfer, HDMA while CPU is HALTed, HDMA register writes during an active transfer, LCD transitions mid-block and CGB double-speed behavior still require hardware-level validation. Its initial white pixels, CGB palette RAM initialization, and DMG grayscale shades are deterministic test/display choices, not claimed as real post-boot LCD colors. No complete scheduler or cycle-accurate CPU-to-bus synchronization exists. Multi-byte stack and memory accesses can partially modify state when an unimplemented bus region rejects a later access. The preliminary mode selection from the cartridge header will eventually belong to boot/hardware initialization. Integration with CorvusSoftware's own API must follow its actual contract when it is available in this repository.

## Development

From the repository root, with the Rust toolchain installed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p corvus-gbc
```

## Running the Emulator

To run a ROM with the SDL2 frontend:

```sh
cargo run --bin corvus_gbc -- <path_to_rom.gb> [--cgb|--dmg]
```

Options:
- `--cgb`: Force CGB mode
- `--dmg`: Force DMG mode (default: auto-detect from cartridge header)

Controls:
- Arrow keys: Direction pad
- A: A button
- S: B button
- Z: Select button
- Enter: Start button
- ESC: Quit

## New Features Added

- **SDL2 Frontend**: Window display with 4x scaling, rendering the PPU framebuffer
- **Boot ROM Emulation**: DMG and CGB boot sequences with proper register initialization
- **Joypad Support**: Full joypad input handling via FF00 register
- **Binary Entry Point**: `corvus_gbc` binary for easy testing

## Hardware references

- [Pan Docs — Memory Map](https://gbdev.io/pandocs/Memory_Map.html)
- [Pan Docs — CGB Registers](https://gbdev.io/pandocs/CGB_Registers.html)
- [Pan Docs — The Cartridge Header](https://gbdev.io/pandocs/The_Cartridge_Header.html)
- [Pan Docs — MBC1](https://gbdev.io/pandocs/MBC1.html)
- [Pan Docs — MBC3](https://gbdev.io/pandocs/MBC3.html)
- [Pan Docs — CPU Instruction Set](https://gbdev.io/pandocs/CPU_Instruction_Set.html)
- [Pan Docs — Timer and Divider Registers](https://gbdev.io/pandocs/Timer_and_Divider_Registers.html)
- [Pan Docs — Timer Obscure Behaviour](https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html)
- [Pan Docs — Interrupts](https://gbdev.io/pandocs/Interrupts.html)
- [Pan Docs — HALT](https://gbdev.io/pandocs/halt.html)
- [Pan Docs — LCD Status Registers](https://gbdev.io/pandocs/STAT.html)
- [Pan Docs — Rendering](https://gbdev.io/pandocs/Rendering.html)
- [Pan Docs — Tile Maps](https://gbdev.io/pandocs/Tile_Maps.html)
- [Pan Docs — Window](https://gbdev.io/pandocs/Window.html)
- [Pan Docs — Object Attribute Memory](https://gbdev.io/pandocs/OAM.html)
- [Pan Docs — LCD Control](https://gbdev.io/pandocs/LCDC.html)
- [Pan Docs — OAM DMA Transfer](https://gbdev.io/pandocs/OAM_DMA_Transfer.html)
- [Pan Docs — CGB VRAM DMA](https://gbdev.io/pandocs/CGB_Registers.html#lcd-vram-dma-transfers)
- [Pan Docs — Accessing VRAM and OAM](https://gbdev.io/pandocs/Accessing_VRAM_and_OAM.html)
- [Pan Docs — Tile Data](https://gbdev.io/pandocs/Tile_Data.html)
- [Pan Docs — Palettes](https://gbdev.io/pandocs/Palettes.html)
- [Pan Docs — Interrupt Sources](https://gbdev.io/pandocs/Interrupt_Sources.html)

All project-owned source files carry CorvusSoftware copyright and AGPL-3.0-only SPDX notices; CI enforces these headers. No copyrighted game ROMs or proprietary firmware are included.

## New Features Added

- **MBC5 Support**: Full MBC5 mapper implementation with up to 512 ROM banks (64 MiB) and 16 RAM banks (128 KiB)
- **Save States**: Basic save state infrastructure with serde/bincode (full component serialization in progress)

## Save States

Save state functionality is partially implemented. The infrastructure is in place with:
- `save_state()` function to serialize machine state
- `load_state()` function to deserialize and restore state
- Version checking to handle format changes

Full serialization of all components (CPU, Bus, PPU, APU, etc.) requires adding `Serialize` and `Deserialize` derives to all core structs, which is in progress.
