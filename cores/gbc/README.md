# Game Boy Color core

This directory contains the independently developed **Game Boy Color core** for CorvusCores, including a path for original Game Boy compatibility. It is **not yet a runnable emulator**.

## Component layout

```text
src/
├── lib.rs              # Crate modules
├── bus/
│   ├── mod.rs          # Partial 16-bit address routing and manual clocking
│   └── timing_tests.rs # Timer/interrupt bus integration tests
├── cpu/
│   ├── mod.rs          # CPU state and public exports
│   ├── registers.rs    # SM83 registers and F flag invariants
│   ├── execute.rs      # Partial base-opcode dispatch and nominal T-cycles
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
│   ├── mod.rs          # Cartridge component exports
│   ├── error.rs        # Cartridge errors
│   ├── header.rs       # Header fields, CGB flag, checksum
│   ├── rom_only.rs     # 32 KiB ROM-only cartridge
│   └── tests.rs        # Synthetic cartridge tests
├── memory/
│   ├── mod.rs
│   ├── wram.rs         # Work RAM banking and echo region
│   └── hram.rs         # High RAM
├── interrupts/
│   ├── mod.rs          # IF/IE, request flags, priority and vectors
│   └── tests.rs        # Register and priority regression tests
├── timer/
│   ├── mod.rs          # DIV/TIMA/TMA/TAC and explicit T-cycle clock
│   └── tests.rs        # Clock sources and overflow timing tests
└── video/
    ├── mod.rs
    ├── vram.rs         # VRAM banks, VBK register
    └── oam.rs          # Object attribute memory
```

Components are grouped by responsibility. Future CPU, PPU, audio, DMA, timer, and input components should receive their own modules when there is functioning code for them. Avoid concentrating unrelated hardware in `lib.rs` or the bus.

## Implemented foundations

- Cartridge header parsing: CGB compatibility, cartridge type and RAM/ROM size fields, header checksum.
- 32 KiB ROM-only cartridge loading and reading. Unsupported mapper types and sizes are rejected.
- Partial bus access with explicit unsupported reads/writes, plus manual T-cycle advance for the timer (`Bus::tick_t_cycles`). The CPU does not yet advance peripherals automatically.
- Timer registers `FF04–FF07`: 16-bit divider, all four TIMA clock selections, falling-edge increments, DIV reset, active TAC source switching, 4-T-cycle delayed TIMA reload and interrupt request. TIMA writes during a pending reload cancel that reload.
- Interrupt registers IF (`FF0F`) and IE (`FFFF`): software and device requests, five priority levels, vector lookup, mask filtering, and acknowledgement. CPU IME and actual interrupt service are **not** implemented.
- Partial SM83 CPU interpreter: register pairs and flag masking, all eight 8-bit accumulator ALU families (ADD, ADC, SUB, SBC, AND, XOR, OR, CP), including register, (HL), and immediate forms; JP/JR branches, and nominal instruction T-cycle counts.
- SM83 transfer module: LD (BC/DE),A and LD A,(BC/DE), auto-increment/decrement LD (HL+/-),A and LD A,(HL+/-), high-memory LDH (a8),A / LDH A,(a8) and LD (C),A / LD A,(C), plus absolute LD (a16),A / LD A,(a16). Transfers preserve flags, use wrapping HL arithmetic, and fail explicitly for unsupported I/O rather than fabricating register behavior.
- Additional arithmetic/flag instructions: DAA, CPL, SCF, CCF, RLCA/RRCA/RLA/RRA, ADD HL,r16, ADD SP,e8, and LD HL,SP+e8. Unit tests exercise carry/half-carry edge cases and two-digit BCD addition/subtraction.
- SM83 stack and control flow: PUSH/POP (including AF masking), CALL/RET (conditional and unconditional), RST vectors, JP HL, LD SP,HL and LD (a16),SP. Stack accesses use the current partial bus and report unmapped-address errors.
- The complete 256-entry **CB-prefixed** SM83 instruction space: RLC, RRC, RL, RR, SLA, SRA, SWAP, SRL, BIT, RES and SET, for registers and `(HL)` where applicable. BIT preserves carry and does not write its operand; `(HL)` variants have distinct nominal timings.
- Explicit CPU errors for unsupported opcodes and unmapped hardware, rather than silently returning success.
- 8 banks of WRAM in CGB mode, including `FF70` (SVBK), fixed bank 0, switchable banks 1–7, and the `E000–FDFF` echo.
- 2 VRAM banks in CGB mode, selected through `FF4F` (VBK).
- HRAM at `FF80–FFFE` and OAM storage at `FE00–FE9F`.
- Basic differences between color mode and original Game Boy compatibility mode for the implemented bank selection.
- Tests with synthetic, self-authored cartridge data and boundary/banking cases.

## Not implemented yet

**No games can execute end to end.** CPU interrupt dispatch (IME/EI/DI/RETI), HALT/STOP, boot behavior, full bus arbitration and cycle-accurate scheduling are not yet implemented. The CGB-specific edge cases for TAC writes and same-cycle timer reload writes still require hardware-reference verification. There is no PPU renderer or timing, APU, joypad, DMA, save states, or MBC1/MBC3/MBC5. Most I/O registers and access restrictions are intentionally not implemented. A bus read returning `None` means its behavior is not modeled yet, **not** that a real console necessarily returns an absent value.

The current zero-initialized RAM and CPU registers are deterministic test starting states, not simulated hardware power-on conditions. CPU cycle counts are nominal instruction timings, not an implemented clock/scheduler. The timer advances only when the caller explicitly invokes `Bus::tick_t_cycles`; this must not be confused with cycle-accurate CPU-to-bus synchronization. Multi-byte stack and memory accesses can partially modify state when an unimplemented bus region rejects a later access. The preliminary mode selection from the cartridge header will eventually belong to boot/hardware initialization. Integration with CorvusSoftware's own API must follow its actual contract when it is available in this repository.

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
- [Pan Docs — CPU Instruction Set](https://gbdev.io/pandocs/CPU_Instruction_Set.html)
- [Pan Docs — Timer and Divider Registers](https://gbdev.io/pandocs/Timer_and_Divider_Registers.html)
- [Pan Docs — Timer Obscure Behaviour](https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html)
- [Pan Docs — Interrupts](https://gbdev.io/pandocs/Interrupts.html)

All project-owned source files carry CorvusSoftware copyright and AGPL-3.0-only SPDX notices; CI enforces these headers. No copyrighted game ROMs or proprietary firmware are included.
