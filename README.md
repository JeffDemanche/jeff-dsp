# jeff-dsp

Collection of DSP plugin experiments — a monorepo for audio plugins built with **JUCE** (host, UI, plugin formats) and **Rust** (realtime DSP), linked via Corrosion.

## Prerequisites

- macOS with Xcode Command Line Tools
- [CMake](https://cmake.org/) 3.25+
- [Rust](https://rustup.rs/) (stable) with `cargo`
- A C++17 compiler (`clang++`)
- [Node.js](https://nodejs.org/) 18+ (for Prism WebView UI assets)

## Quick start

Build UI + plugin in one step:

```bash
./scripts/build_with_ui.sh prism
```

Open the Standalone app:

```bash
./scripts/open_standalone.sh prism
```

Plugin-only build (assumes UI assets already built when needed):

```bash
./scripts/build.sh prism
```

Build everything (Release):

```bash
./scripts/build.sh
```

Or with CMake presets:

```bash
cmake --preset release
cmake --build --preset release --target Prism_VST3
```

Artefacts land under `build/release/plugins/prism/Prism_artefacts/`.

## Layout

| Path | Purpose |
|------|---------|
| `plugins/` | One directory per plugin project |
| `crates/` | Shared Rust crates |
| `modules/` | Shared custom JUCE modules |
| `cmake/` | Repo-level CMake helpers |
| `docs/` | Architecture and decision records (humans + agents) |
| `scripts/` | Build helpers |

## Documentation

Start at [`docs/README.md`](docs/README.md). Agents: see [`AGENTS.md`](AGENTS.md).
