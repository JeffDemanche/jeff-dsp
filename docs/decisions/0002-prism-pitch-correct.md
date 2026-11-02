# 0002 Prism phase-1 pitch correct (`crates/pitch`)

- Status: Accepted
- Date: 2026-11-01

## Context

Prism’s product direction is a “prismizer”: each MIDI voice snaps a monophonic audio signal to that pitch. That needs shared pitch detection and per-voice shifting. We already have a proven low-latency stack in Vamp (`pitch_correct_low_latency`: causal YIN + TD-PSOLA). Phase 1 should ship a single-voice, key-constrained corrector first, with module boundaries that the full prismizer can reuse.

## Decision

- Add a shared Rust crate `crates/pitch` for the pitch stack (not bloating `shared_dsp`).
- Port Vamp low-latency behavior/constants into modular pieces: `yin`, `psola`, `scale`, `pitch_math`, `delay_line`, `smoothed`, plus a `PitchTarget` trait and composing `Corrector`.
- Phase 1 uses `ScalePitchTarget` (chromatic / major / natural minor). MIDI note targets (`MidiNoteTarget`, N× PSOLA) are deferred.
- Prism’s Rust `staticlib` is a thin FFI shell over `Corrector<ScalePitchTarget>`; JUCE owns APVTS, latency reporting, and state.
- No MIDI input on Prism in phase 1.

## Consequences

- Algorithms live in `crates/pitch` and can be reused by other plugins without copying.
- Prismizer follow-up should share one YIN detector and spawn one PSOLA (and target) per MIDI voice.
- Numerical behavior should stay aligned with Vamp LL; intentional divergences need docs.
- PDC latency ≈ one period at `minF0Hz` (same model as Vamp LL).
