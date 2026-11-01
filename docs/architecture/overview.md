# Architecture overview

## Goals

- Multiple independent plugin projects under one repo
- Shared C++ (JUCE modules) and Rust crates
- One root build that can compile **one** or **all** plugins to JUCE formats (VST3, AU, Standalone, …)

## Layering

```text
Host / DAW
    └── JUCE plugin format wrappers (VST3 / AU / Standalone)
            └── JUCE AudioProcessor + Editor (C++)
                    └── C FFI
                            └── Rust DSP staticlib (per plugin)
                                    └── shared_dsp (and other crates/)
```

- **JUCE** owns plugin lifecycle, buses, state blobs, UI, and format packaging.
- **Rust** owns realtime DSP. Crossing the boundary is a small C ABI (`extern "C"` + header under each plugin’s `rust/include/`).
- **Corrosion** imports each plugin’s `Cargo.toml` as a CMake target and links the `staticlib` into the JUCE target.

## Repository map

| Path | Role |
|------|------|
| `plugins/<name>/` | One plugin: `CMakeLists.txt`, `Source/` (C++), optional `rust/` crate |
| `crates/` | Shared Rust libraries (Cargo workspace members) |
| `modules/` | Shared custom JUCE modules (`juce_add_module`); auto-discovered if a `juce_module_info` file exists |
| `cmake/` | `Dependencies.cmake` (FetchContent JUCE + Corrosion), `AddPlugin.cmake` (`dsp_add_plugin`) |
| `scripts/build.sh` | Convenience wrapper for configure + build |
| `docs/` | Architecture + ADRs |

## Build entry points

```bash
./scripts/build.sh                 # all plugins, Release
./scripts/build.sh prism           # one plugin
./scripts/build.sh prism Debug

cmake --preset release
cmake --build --preset release --target Prism_VST3
```

Filter plugins at configure time with `-DDSP_PLUGINS=prism` (semicolon-separated list). Empty means every `plugins/*/CMakeLists.txt`.

JUCE and Corrosion are downloaded via FetchContent on first configure (pinned versions in `cmake/Dependencies.cmake`).

## Adding a plugin

1. Create `plugins/<name>/` with `CMakeLists.txt` calling `dsp_add_plugin(...)`.
2. Add JUCE `Source/` files (processor + editor at minimum).
3. If using Rust DSP: add `plugins/<name>/rust` as a workspace member in the root `Cargo.toml`, expose a C header, set `RUST_CRATE` / `RUST_TARGET` / `RUST_INCLUDE` on `dsp_add_plugin`.
4. Assign unique `PLUGIN_CODE` (and keep manufacturer code consistent unless intentionally changing identity).
5. Document non-obvious choices in an ADR if they affect architecture.

## Shared modules

- **Rust:** add a crate under `crates/`, list it in the workspace, depend from plugin crates.
- **JUCE:** add a folder under `modules/<ModuleName>/` with a valid `juce_module_info` and module sources; the root `CMakeLists.txt` will `juce_add_module` it automatically. Link from the plugin with `target_link_libraries(<Plugin> PRIVATE <ModuleName>)` as needed.
