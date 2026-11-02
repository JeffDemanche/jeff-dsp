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
                    ├── WebView UI (optional; Vite/React assets via BinaryData)
                    └── C FFI
                            └── Rust DSP staticlib (per plugin)
                                    └── crates/pitch, shared_dsp, …
```

- **JUCE** owns plugin lifecycle, buses, state blobs, UI (native or WebView), and format packaging.
- **Rust** owns realtime DSP. Crossing the boundary is a small C ABI (`extern "C"` + header under each plugin’s `rust/include/`).
- **Corrosion** imports each plugin’s `Cargo.toml` as a CMake target and links the `staticlib` into the JUCE target.

## Repository map

| Path | Role |
|------|------|
| `plugins/<name>/` | One plugin: `CMakeLists.txt`, `Source/` (C++), optional `rust/` crate, optional `ui/` WebView frontend |
| `crates/` | Shared Rust libraries (Cargo workspace members) |
| `crates/pitch/` | Shared pitch DSP (YIN, TD-PSOLA, scale snap, corrector, Prismizer multi-voice) |
| `crates/shared_dsp/` | Tiny generic DSP helpers |
| `modules/` | Shared custom JUCE modules (`juce_add_module`); auto-discovered if a `juce_module_info` file exists |
| `cmake/` | `Dependencies.cmake` (FetchContent JUCE + Corrosion), `AddPlugin.cmake` (`dsp_add_plugin`) |
| `scripts/build.sh` | Convenience wrapper for configure + build |
| `docs/` | Architecture + ADRs |

## Build entry points

```bash
./scripts/build_with_ui.sh prism   # UI assets + plugin (preferred for Prism)
./scripts/build.sh                 # all plugins, Release (no UI step)
./scripts/build.sh prism           # one plugin
./scripts/build.sh prism Debug
./scripts/open_standalone.sh prism # open built Standalone .app

cmake --preset release
cmake --build --preset release --target Prism_VST3
```

Filter plugins at configure time with `-DDSP_PLUGINS=prism` (semicolon-separated list). Empty means every `plugins/*/CMakeLists.txt`.

JUCE and Corrosion are downloaded via FetchContent on first configure (pinned versions in `cmake/Dependencies.cmake`).

### macOS Standalone audio input

`dsp_add_plugin` sets `MICROPHONE_PERMISSION_ENABLED` so Standalone Info.plists include `NSMicrophoneUsageDescription`. Without that key, macOS will not prompt and the app will not appear under **System Settings → Privacy & Security → Microphone**.

`scripts/open_standalone.sh` ad-hoc codesigns the `.app` with the bundle id before launch so TCC can track it. On first open, allow the microphone prompt (or enable the app in that Privacy list). Also uncheck **Mute audio input** in Options → Audio/MIDI Settings.

## Prism

Prism is a stereo **audio effect that accepts MIDI**. Shared YIN detects the input f0; each held MIDI note drives a TD-PSOLA voice. Optional `dryCorrect` scale-corrects the underlay (`dryLevel`); `mix` is the MIDI voices level. See ADR 0002 / 0004.

The editor pushes a `prismMetering` event (~30 Hz) to the WebView for the pitch-map visualizer (input f0 → active MIDI targets).

## Prism WebView UI

Prism uses a JUCE 8 WebView editor. Frontend lives in `plugins/prism/ui/` (Vite + React + TypeScript).

```bash
cd plugins/prism/ui
npm install
npm run build          # writes dist/ for BinaryData
# optional hot reload:
npm run dev            # http://localhost:5173 — enable via PRISM_UI_DEV_SERVER in Debug
```

Then build the plugin as usual. CMake packs `ui/dist` into BinaryData when present; without a prior `npm run build`, the editor falls back to a minimal embedded placeholder page.

Debug hot reload: define `PRISM_UI_DEV_SERVER=1` (or set env `PRISM_UI_DEV_SERVER=1`) so the WebView loads Vite instead of BinaryData.

## Adding a plugin

1. Create `plugins/<name>/` with `CMakeLists.txt` calling `dsp_add_plugin(...)`.
2. Add JUCE `Source/` files (processor + editor at minimum).
3. If using Rust DSP: add `plugins/<name>/rust` as a workspace member in the root `Cargo.toml`, expose a C header, set `RUST_CRATE` / `RUST_TARGET` / `RUST_INCLUDE` on `dsp_add_plugin`.
4. If using WebView UI: set `WEB_UI TRUE`, add `ui/` frontend, wire BinaryData + `WebBrowserComponent` (see ADR 0003).
5. Assign unique `PLUGIN_CODE` (and keep manufacturer code consistent unless intentionally changing identity).
6. Document non-obvious choices in an ADR if they affect architecture.

## Shared modules

- **Rust:** add a crate under `crates/`, list it in the workspace, depend from plugin crates. Pitch algorithms belong in `crates/pitch` (ADR 0002).
- **JUCE:** add a folder under `modules/<ModuleName>/` with a valid `juce_module_info` and module sources; the root `CMakeLists.txt` will `juce_add_module` it automatically. Link from the plugin with `target_link_libraries(<Plugin> PRIVATE <ModuleName>)` as needed.
