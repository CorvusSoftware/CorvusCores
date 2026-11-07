# CorvusCores

**CorvusCores** is a project by CorvusSoftware to develop and maintain retro console emulation cores in a single repository.

Each core implements the hardware behavior of its target system while exposing a consistent interface through the project's own API. CorvusCores is **not based on Libretro**.

## Project goals

- Keep emulation cores independently maintainable and testable.
- Prioritize correctness, deterministic behavior, and reproducible tests.
- Share code only where hardware behavior and maintenance needs justify it.
- Keep the public core API separate from console-specific implementations.

## Project status

CorvusCores is at an early stage. The repository is being established; **no console support or compatibility level is claimed yet**.

The Game Boy Color is a planned target, with support for original Game Boy software through appropriate compatibility handling. Additional systems can be introduced as their implementations become ready.

## Repository organization

The intended separation of responsibilities is:

- `cores/` — console-specific emulation cores.
- `crates/` — shared components and API integration, where appropriate.
- `frontends/` — optional applications that consume the project's API.
- `tests/` — integration, regression, and hardware conformance tests.

These directories are a **planned organization**, not a claim that their implementations already exist. Each component should be added when it has a concrete purpose.

## Development

Implementation details, build instructions, and test commands will be documented with the corresponding code as it is introduced. No working build or emulator is implied by the current project status.

Contributors and coding agents should read [AGENTS.md](AGENTS.md) before making changes. Please avoid committing copyrighted game ROMs, proprietary firmware, or other assets without redistribution rights.

## License

CorvusCores is licensed under the **GNU Affero General Public License, version 3.0 only (AGPL-3.0-only)**. See [LICENSE](LICENSE) for the full terms.
