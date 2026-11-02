# 0002 Prism phase-1 pitch correct (`crates/pitch`)

- Status: Accepted
- Date: 2026-11-01

## Context

Prism’s product direction is a “prismizer”: each MIDI voice snaps a monophonic audio signal to that pitch. That needs shared pitch detection and per-voice shifting. We already have a proven low-latency stack in Vamp (`pitch_correct_low_latency`: causal YIN + TD-PSOLA). Phase 1 should ship a single-voice, key-constrained corrector first, with module boundaries that the full prismizer can reuse.

## Decision

- Add a shared Rust crate `crates/pitch` for the pitch stack (not bloating `shared_dsp`).
- Port Vamp low-latency behavior/constants into modular pieces: `yin`, `psola`, `scale`, `pitch_math`, `delay_line`, `smoothed`, plus a `PitchTarget` trait and composing `Corrector`.
- Phase 1 used `ScalePitchTarget` (chromatic / major / natural minor) behind `Corrector`. MIDI multi-voice landed in ADR 0004 (`Prismizer`).
- Prism’s Rust `staticlib` is a thin FFI shell; JUCE owns APVTS, MIDI, latency reporting, and state.

## Consequences

- Algorithms live in `crates/pitch` and can be reused by other plugins without copying.
- Multi-voice MIDI prismizer: see [0004-prism-midi-voices.md](0004-prism-midi-voices.md) (shared YIN + N× PSOLA).
- Numerical behavior should stay aligned with Vamp LL; intentional divergences need docs.
- PDC latency ≈ one period at `minF0Hz` (same model as Vamp LL).
