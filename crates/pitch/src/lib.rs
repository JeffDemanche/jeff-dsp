//! Shared realtime pitch DSP: YIN detection, TD-PSOLA shifting, scale snap, corrector.
//!
//! Ported from Vamp `pitch_correct_low_latency` with modular boundaries for Prismizer reuse.

pub mod corrector;
pub mod delay_line;
pub mod pitch_math;
pub mod prismizer;
pub mod psola;
pub mod scale;
pub mod smoothed;
pub mod target;
pub mod yin;

pub use corrector::{Corrector, CorrectorParams};
pub use delay_line::DelayLine;
pub use pitch_math::{clamp_ratio_to_correction_range, hz_to_midi, midi_to_hz, one_pole_coeff};
pub use prismizer::{Prismizer, PrismizerMetering, PrismizerParams, MAX_VOICES};
pub use psola::PsolaShifter;
pub use scale::{nearest_scale_midi, quantize_scale_degree_with_hysteresis, ScaleKind};
pub use smoothed::SmoothedValue;
pub use target::{MidiNoteTarget, PitchTarget, ScalePitchTarget};
pub use yin::{YinDetector, SCHEMA_MAX_F0_HZ, SCHEMA_MIN_F0_HZ};

/// Default `minF0Hz` (detection floor / PDC latency tradeoff).
pub const DEFAULT_MIN_F0_HZ: f32 = 50.0;
/// Upper detect limit (independent of min bound).
pub const MAX_F0_HZ: f32 = 1200.0;
/// UI / param smoothing time (ms).
pub const PARAM_SMOOTH_MS: f32 = 25.0;
