# 0001 Monorepo build stack (JUCE + Rust)

- Status: Accepted
- Date: 2026-11-01

## Context

This repo will host multiple experimental DSP plugins. We need a single place for shared code, a way to build one or all plugins to standard plugin formats, and a clear split between host/UI concerns and realtime DSP. The authoring environment already has CMake, Rust, and Apple clang available.

## Decision

- Organize the repo as a **plugin monorepo**: `plugins/` for products, `crates/` for shared Rust, `modules/` for shared JUCE modules.
- Use **JUCE 8** (FetchContent, pinned tag) as the plugin shell and format exporter (VST3, AU, Standalone on macOS).
- Implement realtime DSP primarily in **Rust**, linked as a `staticlib` through **Corrosion** and a small C ABI.
- Drive builds from a **root CMake** project with `dsp_add_plugin`, `DSP_PLUGINS` filtering, and `CMakePresets` (`debug` / `release`), plus `scripts/build.sh`.
- Keep architecture rationale in `docs/` (overview + ADRs); instruct agents via `AGENTS.md` and `.cursor/rules/document-decisions.mdc`.

## Consequences

- First configure requires network access to fetch JUCE and Corrosion.
- Plugin authors must maintain a C header matching the Rust `extern "C"` surface.
- Adding a plugin is mostly copy-scaffold + unique `PLUGIN_CODE` + optional Cargo workspace membership.
- Projucer is not part of the workflow; CMake is the source of truth.
- Windows/Linux are not first-class verified yet but the CMake layout is intended to be portable.
