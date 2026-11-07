# AGENTS.md — CorvusCores

These instructions apply to the entire repository, including every future emulation core and shared component.

## Project intent

CorvusCores is a **single repository for retro console emulation cores**. Each core must remain independently maintainable and integrate through **CorvusSoftware's own API**. Do not introduce Libretro, Libretro wrappers, or a Libretro-shaped internal architecture unless explicitly requested.

The project's public entry point is [README.md](README.md). Keep that README stable: change it only when a real project goal, user-visible capability, repository contract, or licensing fact has changed. Put detailed implementation notes closer to the relevant code.

## Architecture

- Use `cores/` for console-specific emulators; **do not rename it to `systems/`**.
- Use `crates/` only for genuinely reusable code and API integration, `frontends/` for consumers, and `tests/` for cross-component validation, if and when those directories are needed.
- Treat the actual API implementation as the source of truth. **Inspect it before designing or modifying a core**; never invent signatures, conventions, or bindings.
- Keep hardware-specific timing, memory maps, buses, interrupts, DMA, and peripheral behavior within the relevant core unless reuse is justified by real equivalence.
- Do not force different machines into an artificial universal CPU, PPU/GPU, or bus abstraction.
- The intended Game Boy family core is `cores/gbc/`, with original Game Boy compatibility handled inside it as appropriate. Distinguish DMG and CGB behavior; do not assume they are identical.
- Do not create empty cores, speculative modules, dependencies, or folders merely to match a proposed roadmap.
- Do not decide between Rust, C, and C++ solely from this document. Follow established repository code and explicit project decisions when implementation begins.

## Emulation correctness

- Prefer documented hardware behavior and reproducible evidence over shortcuts or guessed constants.
- Keep execution deterministic where practical. Make cycle accounting, interrupt sequencing, memory access side effects, and save-state compatibility explicit.
- Separate emulated state from presentation, host I/O, and debugging facilities.
- Profile before claiming speed improvements; benchmark representative workloads and disclose trade-offs.
- When fixing emulation behavior, add a focused regression test when possible.

## Working rules

1. Read the relevant source, API contract, and tests before editing.
2. Make the smallest coherent change; preserve unrelated behavior and existing interfaces.
3. Follow existing language, style, tooling, and error-handling conventions rather than introducing alternatives without a concrete need.
4. Add tests for new behavior, boundary conditions, and plausible regressions. Use suitable public-domain, freely redistributable, or self-authored test assets.
5. **Never commit commercial ROMs, proprietary BIOS/firmware, keys, or assets without explicit redistribution rights.**
6. Run available formatting, linting, compilation, and targeted tests. Report exact commands and results; never claim checks passed if they were not run.
7. If the repository is not yet buildable, say so clearly rather than inventing setup instructions or success claims.

## Licensing

The project is distributed under **AGPL-3.0-only**; see [LICENSE](LICENSE). Preserve the full license text and applicable notices.

**Every newly created or modified CorvusSoftware-owned source file must carry an explicit SPDX header at the top.** For Rust source files, use:

```rust
// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only
```

For TOML, YAML, Python, and shell scripts, use the equivalent `#` comment syntax. Preserve the actual copyright years and owners of third-party code; **never falsely reassign authorship or overwrite upstream license notices**. The full license remains in the repository's root `LICENSE` file, not duplicated in every source.

Check the licenses and redistribution permissions of third-party code and assets before adding them, and document any required attributions. Never paste code from an incompatible project without appropriate authorization.

## Documentation and handoff

Keep comments focused on non-obvious hardware details and invariants. Document design decisions where they are implemented. When reporting work, distinguish what changed, what was verified, and what remains unverified.
