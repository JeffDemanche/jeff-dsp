//! Pitch target providers (scale snap and MIDI note).

use crate::pitch_math::{hz_to_midi, midi_to_hz};
use crate::scale::{quantize_scale_degree_with_hysteresis, ScaleKind};

/// Runtime inputs shared by target implementations.
#[derive(Debug, Clone, Copy)]
pub struct TargetContext {
    pub detected_hz: f32,
    pub pitch_lf: f32,
    pub pitch_hf: f32,
    pub reference_hz: f32,
    pub correction_amount: f32,
    pub correction_range_semitones: f32,
    pub vibrato_preserve: f32,
    pub quantize_dead_zone_cents: f32,
    pub quantize_enabled: bool,
}

/// Strategy for turning detected pitch into a correction target.
pub trait PitchTarget {
    fn reset(&mut self);
    /// Returns target Hz (0 = no target / pass-through handling upstream).
    fn target_hz(&mut self, ctx: &TargetContext) -> f32;
}

/// Key/scale constrained snap (phase-1 Prism).
pub struct ScalePitchTarget {
    key_semitone: u8,
    scale: ScaleKind,
    last_quantized_midi: f32,
}

impl ScalePitchTarget {
    pub fn new(key_semitone: u8, scale: u8) -> Self {
        Self {
            key_semitone: key_semitone.min(11),
            scale: ScaleKind::from_u8(scale),
            last_quantized_midi: 0.0,
        }
    }

    pub fn set_key_scale(&mut self, key_semitone: u8, scale: u8) {
        self.key_semitone = key_semitone.min(11);
        self.scale = ScaleKind::from_u8(scale);
    }
}

impl Default for ScalePitchTarget {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl PitchTarget for ScalePitchTarget {
    fn reset(&mut self) {
        self.last_quantized_midi = 0.0;
    }

    fn target_hz(&mut self, ctx: &TargetContext) -> f32 {
        if ctx.detected_hz <= 0.0 {
            return 0.0;
        }
        let vibrato = ctx.detected_hz - ctx.pitch_hf;
        let intended = ctx.pitch_lf;
        let intended_midi = hz_to_midi(intended, ctx.reference_hz);
        let quantized = if ctx.quantize_enabled {
            match quantize_scale_degree_with_hysteresis(
                intended_midi,
                self.key_semitone,
                self.scale,
                ctx.correction_range_semitones,
                self.last_quantized_midi,
            ) {
                Some(degree) => {
                    self.last_quantized_midi = degree;
                    let cents = (degree - intended_midi) * 100.0;
                    if cents.abs() < ctx.quantize_dead_zone_cents {
                        intended_midi
                    } else {
                        degree
                    }
                }
                None => {
                    self.last_quantized_midi = 0.0;
                    intended_midi
                }
            }
        } else {
            self.last_quantized_midi = 0.0;
            intended_midi
        };
        let mut target = midi_to_hz(quantized, ctx.reference_hz);
        target += vibrato * ctx.vibrato_preserve;
        let full_target = intended + (target - intended) * ctx.correction_amount;
        if full_target <= 0.0 {
            0.0
        } else {
            full_target
        }
    }
}

/// Snap detected pitch toward a fixed MIDI note (prismizer voice target).
pub struct MidiNoteTarget {
    pub midi_note: f32,
}

impl MidiNoteTarget {
    pub fn new(midi_note: f32) -> Self {
        Self { midi_note }
    }

    pub fn set_midi_note(&mut self, midi_note: f32) {
        self.midi_note = midi_note;
    }
}

impl PitchTarget for MidiNoteTarget {
    fn reset(&mut self) {}

    fn target_hz(&mut self, ctx: &TargetContext) -> f32 {
        if ctx.detected_hz <= 0.0 {
            return 0.0;
        }
        let vibrato = ctx.detected_hz - ctx.pitch_hf;
        let intended = ctx.pitch_lf;
        let mut target = midi_to_hz(self.midi_note, ctx.reference_hz);
        target += vibrato * ctx.vibrato_preserve;
        let full_target = intended + (target - intended) * ctx.correction_amount;
        if full_target <= 0.0 {
            0.0
        } else {
            full_target
        }
    }
}
