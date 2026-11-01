# Agent instructions

This repository is a JUCE + Rust DSP plugin monorepo. Read [`docs/README.md`](docs/README.md) before making structural or product changes.

## Documentation duty

When you make a **product** or **engineering architecture** decision (repo layout, build stack, plugin identity, FFI boundaries, shared module design, format targets, dependency strategy, etc.):

1. Add or update an Architecture Decision Record under [`docs/decisions/`](docs/decisions/) using the format in that folder's README.
2. Update [`docs/architecture/`](docs/architecture/) if the system shape or how-to guidance changed.
3. Keep the root [`README.md`](README.md) accurate for prerequisites and first-build steps only; deep detail belongs in `docs/`.

Do not leave architecture decisions only in chat history or commit messages.

## Build orientation

- Plugins live under `plugins/<name>/` (JUCE C++ shell + optional Rust DSP crate).
- Shared Rust code lives under `crates/`.
- Shared custom JUCE modules live under `modules/`.
- Repo-level build: `./scripts/build.sh` or CMake presets (`debug` / `release`).
- DSP realtime code prefers Rust (`staticlib` + C FFI) linked via Corrosion; JUCE owns host/UI/formats.
