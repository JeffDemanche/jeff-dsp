# 0004 Prism MIDI multi-voice prismizer

- Status: Accepted
- Date: 2026-11-01

## Context

ADR 0002 shipped Prism as a single-voice scale corrector and deferred MIDI. The product goal is a prismizer: each held MIDI note becomes a pitch-shifted copy of the monophonic input. Phase 1 boundaries (`crates/pitch`, shared YIN, per-voice PSOLA) were chosen so this follow-up could reuse them.

## Decision

- Prism remains a **stereo audio effect that accepts MIDI** (`NEEDS_MIDI_INPUT TRUE`, not a synth). Bus layouts: mono/mono, stereo/stereo, or **mono→stereo**; stereo→mono rejected. Before DSP, channel 0 is copied to channel 1 when outputs are stereo so a single mic (e.g. Scarlett input 1) reaches both ears. Pitch remains mono-source; planar stereo is kept for future stereo FX.
- DSP engine is `pitch::Prismizer`: one shared YIN + LF/HF split, up to **8** `MidiNoteTarget` + `PsolaShifter` voices, one optional scale-correct dry underlay sharing the same tracker.
- MIDI notes map to **exact note Hz** (no octave fold yet). PSOLA hard ratio clamp widened to **0.25–4.0** (±2 octaves) as a temporary first pass.
- New params: `dryCorrect` (bool, default on) and `dryLevel`. Existing `mix` is the MIDI voices level (UI label “Voices”). Key/scale/range/dead-zone apply only to dryCorrect.
- Per-voice **linear ADSR** (`attackMs` / `decayMs` / `sustain` / `releaseMs`) multiplies each voice’s pitch-shifted audio after the PSOLA path fade. Defaults: 5 ms / 0 ms / 1.0 / 80 ms.
- `note_off` enters **Release** (voice keeps processing until env ≈ 0); MIDI All Notes Off does the same; All Sound Off hard-mutes. Steal prefers oldest releasing voice, else oldest by age.
- Voice summing is **unnormalized** (each active voice at full level × env × Voices gain) so new keys feel independent. Polyphony can sum louder — rely on Voices / host gain / envelope.
- Metering snapshot (`detected_hz`, dry target, active MIDI targets) crosses FFI; the editor emits `prismMetering` to the WebView at ~30 Hz for the pitch-map visualizer.

## Consequences

- Hosts must route MIDI into Prism for multi-voice behavior; with no notes, only the dry / dryCorrect path is heard.
- Long release times occupy voice slots and increase steal pressure under dense polyphony.
- Large MIDI intervals may still sound rough until octave policy / formant work lands.
- ADR 0002’s “MIDI deferred” bullet is superseded by this record for Prism product scope.
- Velocity→peak and exponential envelope curves are deferred.
