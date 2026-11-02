//! Multi-voice prismizer: shared YIN + per-MIDI-note TD-PSOLA + optional scale dry.

use crate::delay_line::DelayLine;
use crate::pitch_math::{clamp_ratio_to_correction_range, one_pole_coeff};
use crate::psola::PsolaShifter;
use crate::smoothed::SmoothedValue;
use crate::target::{MidiNoteTarget, PitchTarget, ScalePitchTarget, TargetContext};
use crate::yin::{YinDetector, SCHEMA_MAX_F0_HZ, SCHEMA_MIN_F0_HZ};
use crate::{DEFAULT_MIN_F0_HZ, PARAM_SMOOTH_MS};

/// Maximum simultaneous MIDI voices.
pub const MAX_VOICES: usize = 8;

const PITCH_LF_SMOOTH_MS: f32 = 15.0;
const PITCH_HF_SMOOTH_MS: f32 = 6.0;
const DEFAULT_QUANTIZE_DEAD_ZONE_CENTS: f32 = 8.0;
const MIN_F0_HISTORY_FOR_SHIFT: usize = 2;
const MAX_RATIO_SLEW_PER_MS: f32 = 0.04;
const PSOLA_PATH_FADE_MS: f32 = 4.0;
/// Keep shifting with last f0 through YIN dropouts; clear LF/HF after this.
const SHIFT_HOLD_MS: f32 = 90.0;

/// User-facing prismizer parameters.
#[derive(Debug, Clone)]
pub struct PrismizerParams {
    pub key_semitone: u8,
    pub scale: u8,
    pub reference_hz: f32,
    pub retune_speed_ms: f32,
    pub correction_amount: f32,
    pub correction_range_semitones: f32,
    pub vibrato_preserve: f32,
    pub formant_follow: f32,
    pub quantize_dead_zone_cents: f32,
    /// MIDI voices output level (legacy id: mix).
    pub mix: f32,
    pub min_f0_hz: f32,
    pub dry_level: f32,
    pub dry_correct: bool,
}

impl Default for PrismizerParams {
    fn default() -> Self {
        Self {
            key_semitone: 0,
            scale: 1,
            reference_hz: 440.0,
            retune_speed_ms: 20.0,
            correction_amount: 1.0,
            correction_range_semitones: 2.0,
            vibrato_preserve: 0.7,
            formant_follow: 0.0,
            quantize_dead_zone_cents: DEFAULT_QUANTIZE_DEAD_ZONE_CENTS,
            mix: 1.0,
            min_f0_hz: DEFAULT_MIN_F0_HZ,
            dry_level: 1.0,
            dry_correct: true,
        }
    }
}

/// Snapshot for UI metering / visualizer.
#[derive(Debug, Clone, Copy)]
pub struct PrismizerMetering {
    pub detected_hz: f32,
    pub dry_target_hz: f32,
    pub num_voices: u32,
    pub voice_midi: [f32; MAX_VOICES],
    pub voice_target_hz: [f32; MAX_VOICES],
}

impl Default for PrismizerMetering {
    fn default() -> Self {
        Self {
            detected_hz: 0.0,
            dry_target_hz: 0.0,
            num_voices: 0,
            voice_midi: [0.0; MAX_VOICES],
            voice_target_hz: [0.0; MAX_VOICES],
        }
    }
}

struct VoiceState {
    active: bool,
    midi_note: u8,
    age: u64,
    target: MidiNoteTarget,
    psola: PsolaShifter,
    corrected_hz: f32,
    applied_ratio: f32,
    psola_path_gain: f32,
    last_target_hz: f32,
}

impl VoiceState {
    fn new(sample_rate: f32) -> Self {
        Self {
            active: false,
            midi_note: 0,
            age: 0,
            target: MidiNoteTarget::new(60.0),
            psola: PsolaShifter::new(sample_rate),
            corrected_hz: 0.0,
            applied_ratio: 1.0,
            psola_path_gain: 0.0,
            last_target_hz: 0.0,
        }
    }

    fn reset_dsp(&mut self) {
        self.target.reset();
        self.psola.reset();
        self.corrected_hz = 0.0;
        self.applied_ratio = 1.0;
        self.psola_path_gain = 0.0;
        self.last_target_hz = 0.0;
    }

    fn deactivate(&mut self) {
        self.active = false;
        self.reset_dsp();
    }
}

struct DryCorrectState {
    target: ScalePitchTarget,
    psola: PsolaShifter,
    corrected_hz: f32,
    applied_ratio: f32,
    psola_path_gain: f32,
    last_target_hz: f32,
}

impl DryCorrectState {
    fn new(sample_rate: f32, key: u8, scale: u8) -> Self {
        Self {
            target: ScalePitchTarget::new(key, scale),
            psola: PsolaShifter::new(sample_rate),
            corrected_hz: 0.0,
            applied_ratio: 1.0,
            psola_path_gain: 0.0,
            last_target_hz: 0.0,
        }
    }

    fn reset(&mut self) {
        self.target.reset();
        self.psola.reset();
        self.corrected_hz = 0.0;
        self.applied_ratio = 1.0;
        self.psola_path_gain = 0.0;
        self.last_target_hz = 0.0;
    }
}

/// Shared-detector multi-voice pitch prismizer.
pub struct Prismizer {
    sample_rate: f32,
    key_semitone: u8,
    scale: u8,
    dry_correct: bool,
    reference_hz: SmoothedValue,
    retune_speed_ms: SmoothedValue,
    correction_amount: SmoothedValue,
    correction_range: SmoothedValue,
    vibrato_preserve: SmoothedValue,
    formant_follow: SmoothedValue,
    quantize_dead_zone_cents: SmoothedValue,
    mix: SmoothedValue,
    min_f0_hz: SmoothedValue,
    dry_level: SmoothedValue,
    detector: YinDetector,
    pitch_lf: f32,
    pitch_hf: f32,
    pitch_lf_coeff: f32,
    pitch_hf_coeff: f32,
    /// Last stable f0 used for shifting through brief unvoiced gaps.
    held_f0_hz: f32,
    /// Samples since `held_f0_hz` was refreshed by a voiced detection.
    held_unvoiced_samples: usize,
    shift_hold_samples: usize,
    /// True once we have enough F0 history to leave unity; sticky until idle clear.
    shift_ready: bool,
    max_ratio_delta_per_sample: f32,
    psola_path_coeff: f32,
    dry_delay_l: DelayLine,
    dry_delay_r: DelayLine,
    dry: DryCorrectState,
    voices: [VoiceState; MAX_VOICES],
    note_age_counter: u64,
    metering: PrismizerMetering,
}

impl Prismizer {
    pub fn new(sample_rate: f32, params: &PrismizerParams) -> Self {
        let sr = sample_rate.max(1.0);
        let dry_cap = ((sr / SCHEMA_MIN_F0_HZ).ceil() as usize) + 16;
        let voices = std::array::from_fn(|_| VoiceState::new(sr));

        Self {
            sample_rate: sr,
            key_semitone: params.key_semitone.min(11),
            scale: params.scale.min(2),
            dry_correct: params.dry_correct,
            reference_hz: SmoothedValue::new(params.reference_hz, sr, PARAM_SMOOTH_MS),
            retune_speed_ms: SmoothedValue::new(params.retune_speed_ms, sr, PARAM_SMOOTH_MS),
            correction_amount: SmoothedValue::new(params.correction_amount, sr, PARAM_SMOOTH_MS),
            correction_range: SmoothedValue::new(
                params.correction_range_semitones,
                sr,
                PARAM_SMOOTH_MS,
            ),
            vibrato_preserve: SmoothedValue::new(params.vibrato_preserve, sr, PARAM_SMOOTH_MS),
            formant_follow: SmoothedValue::new(params.formant_follow, sr, PARAM_SMOOTH_MS),
            quantize_dead_zone_cents: SmoothedValue::new(
                params.quantize_dead_zone_cents,
                sr,
                PARAM_SMOOTH_MS,
            ),
            mix: SmoothedValue::new(params.mix, sr, PARAM_SMOOTH_MS),
            min_f0_hz: SmoothedValue::new(params.min_f0_hz, sr, PARAM_SMOOTH_MS),
            dry_level: SmoothedValue::new(params.dry_level, sr, PARAM_SMOOTH_MS),
            detector: YinDetector::new(sr),
            pitch_lf: 0.0,
            pitch_hf: 0.0,
            pitch_lf_coeff: one_pole_coeff(sr, PITCH_LF_SMOOTH_MS),
            pitch_hf_coeff: one_pole_coeff(sr, PITCH_HF_SMOOTH_MS),
            held_f0_hz: 0.0,
            held_unvoiced_samples: 0,
            shift_hold_samples: ((SHIFT_HOLD_MS / 1000.0) * sr).round().max(1.0) as usize,
            shift_ready: false,
            max_ratio_delta_per_sample: MAX_RATIO_SLEW_PER_MS / (sr / 1000.0).max(1.0),
            psola_path_coeff: one_pole_coeff(sr, PSOLA_PATH_FADE_MS),
            dry_delay_l: DelayLine::new(dry_cap),
            dry_delay_r: DelayLine::new(dry_cap),
            dry: DryCorrectState::new(sr, params.key_semitone.min(11), params.scale.min(2)),
            voices,
            note_age_counter: 0,
            metering: PrismizerMetering::default(),
        }
    }

    pub fn reported_latency_samples(&self) -> u32 {
        let min_f0 = self.min_f0_hz.value().clamp(SCHEMA_MIN_F0_HZ, SCHEMA_MAX_F0_HZ);
        (self.sample_rate / min_f0).round().max(1.0) as u32
    }

    pub fn metering(&self) -> PrismizerMetering {
        self.metering
    }

    pub fn set_params(&mut self, params: &PrismizerParams) {
        self.key_semitone = params.key_semitone.min(11);
        self.scale = params.scale.min(2);
        self.dry_correct = params.dry_correct;
        self.dry
            .target
            .set_key_scale(self.key_semitone, self.scale);
        self.reference_hz
            .set_target(params.reference_hz.clamp(400.0, 480.0));
        self.retune_speed_ms
            .set_target(params.retune_speed_ms.clamp(0.0, 500.0));
        self.correction_amount
            .set_target(params.correction_amount.clamp(0.0, 1.0));
        self.correction_range
            .set_target(params.correction_range_semitones.clamp(0.0, 12.0));
        self.vibrato_preserve
            .set_target(params.vibrato_preserve.clamp(0.0, 1.0));
        self.formant_follow
            .set_target(params.formant_follow.clamp(0.0, 1.0));
        self.quantize_dead_zone_cents
            .set_target(params.quantize_dead_zone_cents.clamp(0.0, 50.0));
        self.mix.set_target(params.mix.clamp(0.0, 1.0));
        self.min_f0_hz
            .set_target(params.min_f0_hz.clamp(SCHEMA_MIN_F0_HZ, SCHEMA_MAX_F0_HZ));
        self.dry_level.set_target(params.dry_level.clamp(0.0, 1.0));
    }

    pub fn reset(&mut self) {
        self.detector.reset();
        self.pitch_lf = 0.0;
        self.pitch_hf = 0.0;
        self.held_f0_hz = 0.0;
        self.held_unvoiced_samples = 0;
        self.shift_ready = false;
        self.dry.reset();
        self.dry_delay_l.clear();
        self.dry_delay_r.clear();
        for voice in &mut self.voices {
            voice.deactivate();
        }
        self.note_age_counter = 0;
        self.metering = PrismizerMetering::default();
    }

    pub fn note_on(&mut self, midi_note: u8, _velocity: f32) {
        let note = midi_note.min(127);
        // Retrigger existing note.
        if let Some(v) = self.voices.iter_mut().find(|v| v.active && v.midi_note == note) {
            self.note_age_counter = self.note_age_counter.wrapping_add(1);
            v.age = self.note_age_counter;
            v.reset_dsp();
            v.target.set_midi_note(note as f32);
            return;
        }

        let slot = self
            .voices
            .iter()
            .position(|v| !v.active)
            .unwrap_or_else(|| {
                self.voices
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, v)| v.age)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });

        self.note_age_counter = self.note_age_counter.wrapping_add(1);
        let voice = &mut self.voices[slot];
        voice.active = true;
        voice.midi_note = note;
        voice.age = self.note_age_counter;
        voice.reset_dsp();
        voice.target.set_midi_note(note as f32);
    }

    pub fn note_off(&mut self, midi_note: u8) {
        let note = midi_note.min(127);
        for voice in &mut self.voices {
            if voice.active && voice.midi_note == note {
                voice.deactivate();
            }
        }
    }

    /// Idle timeout only: clear LF/HF / dry scale state. Keep MIDI voice ratios.
    fn clear_idle_pitch_state(&mut self) {
        self.pitch_lf = 0.0;
        self.pitch_hf = 0.0;
        self.held_f0_hz = 0.0;
        self.held_unvoiced_samples = 0;
        self.shift_ready = false;
        self.dry.target.reset();
        self.dry.corrected_hz = 0.0;
        self.dry.applied_ratio = 1.0;
        self.dry.last_target_hz = 0.0;
        // Do not reset MIDI voice corrected_hz / applied_ratio — note-off handles that.
    }

    /// Real F0 jump while already voiced: reseat LF/HF + dry corrector, leave MIDI ratios.
    fn reseat_lf_hf(&mut self, new_hz: f32) {
        self.pitch_lf = new_hz;
        self.pitch_hf = new_hz;
        self.dry.target.reset();
        self.dry.corrected_hz = 0.0;
    }

    fn update_pitch_split(&mut self, f0: f32) {
        if f0 <= 0.0 {
            return;
        }
        if self.pitch_lf <= 0.0 {
            self.pitch_lf = f0;
            self.pitch_hf = f0;
            return;
        }
        self.pitch_lf += self.pitch_lf_coeff * (f0 - self.pitch_lf);
        self.pitch_hf += self.pitch_hf_coeff * (f0 - self.pitch_hf);
    }

    fn update_held_f0(&mut self) {
        let detected = self.detector.detected_hz;
        if detected > 0.0 {
            self.held_f0_hz = detected;
            self.held_unvoiced_samples = 0;
            if self.detector.f0_history_len >= MIN_F0_HISTORY_FOR_SHIFT {
                self.shift_ready = true;
            }
        } else if self.held_f0_hz > 0.0 {
            self.held_unvoiced_samples = self.held_unvoiced_samples.saturating_add(1);
            if self.held_unvoiced_samples >= self.shift_hold_samples {
                self.clear_idle_pitch_state();
            }
        }
    }

    fn update_corrected_hz(corrected: &mut f32, target: f32, sample_rate: f32, retune_ms: f32) {
        if target <= 0.0 {
            return;
        }
        if *corrected <= 0.0 {
            *corrected = target;
            return;
        }
        let coeff = one_pole_coeff(sample_rate, retune_ms.max(0.5));
        *corrected += coeff * (target - *corrected);
    }

    fn slew_ratio(
        applied: &mut f32,
        target_ratio: f32,
        max_delta: f32,
    ) -> f32 {
        let delta = (target_ratio - *applied).clamp(-max_delta, max_delta);
        *applied += delta;
        *applied
    }

    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        let max_delta = self.max_ratio_delta_per_sample;
        let path_coeff = self.psola_path_coeff;
        let sample_rate = self.sample_rate;
        let dry_correct = self.dry_correct;

        for i in 0..frames {
            let reference = self.reference_hz.next();
            let retune_ms = self.retune_speed_ms.next();
            let amount = self.correction_amount.next();
            let range = self.correction_range.next();
            let vibrato_keep = self.vibrato_preserve.next();
            let formant_follow = self.formant_follow.next().clamp(0.0, 1.0);
            let dead_zone_cents = self.quantize_dead_zone_cents.next().clamp(0.0, 50.0);
            let voices_level = self.mix.next().clamp(0.0, 1.0);
            let dry_level = self.dry_level.next().clamp(0.0, 1.0);
            let min_f0 = self
                .min_f0_hz
                .next()
                .clamp(SCHEMA_MIN_F0_HZ, SCHEMA_MAX_F0_HZ);
            let latency = (sample_rate / min_f0).max(1.0);

            let dry_l = left[i];
            let dry_r = right[i];

            let delayed_l = self.dry_delay_l.read_at(latency);
            let delayed_r = self.dry_delay_r.read_at(latency);
            self.dry_delay_l.write_sample(dry_l);
            self.dry_delay_r.write_sample(dry_r);

            let mono = (dry_l + dry_r) * 0.5;
            let prev_hz = self.detector.detected_hz;
            self.detector.process_sample(mono, min_f0, true);

            // Real jump while already voiced: reseat LF/HF only (keep MIDI ratios).
            // Skip artificial 0→voiced jump_reseat so we don't thrash after short gaps.
            if self.detector.jump_reseat && prev_hz > 0.0 && self.detector.detected_hz > 0.0 {
                self.reseat_lf_hf(self.detector.detected_hz);
            }

            self.update_held_f0();
            let shift_f0 = self.held_f0_hz;
            if self.detector.detected_hz > 0.0 {
                self.update_pitch_split(self.detector.detected_hz);
            }

            let pitch_lf = if self.pitch_lf > 0.0 {
                self.pitch_lf
            } else {
                shift_f0
            };
            let pitch_hf = if self.pitch_hf > 0.0 {
                self.pitch_hf
            } else {
                shift_f0
            };

            let ctx = TargetContext {
                detected_hz: shift_f0,
                pitch_lf,
                pitch_hf,
                reference_hz: reference,
                correction_amount: amount,
                correction_range_semitones: range,
                vibrato_preserve: vibrato_keep,
                quantize_dead_zone_cents: dead_zone_cents,
                quantize_enabled: true,
            };

            let force_unity = !self.shift_ready || shift_f0 <= 0.0;
            let period = if shift_f0 > 0.0 {
                sample_rate / shift_f0
            } else {
                latency
            };

            // --- Dry / dryCorrect underlay ---
            let (under_l, under_r, dry_target_hz) = if dry_correct {
                let target = self.dry.target.target_hz(&ctx);
                self.dry.last_target_hz = target;
                Self::update_corrected_hz(
                    &mut self.dry.corrected_hz,
                    target,
                    sample_rate,
                    retune_ms,
                );

                let target_ratio = if force_unity {
                    1.0
                } else if self.dry.corrected_hz > 0.0 && shift_f0 > 0.0 {
                    clamp_ratio_to_correction_range(self.dry.corrected_hz / shift_f0, range)
                } else {
                    1.0
                };
                let pitch_ratio =
                    Self::slew_ratio(&mut self.dry.applied_ratio, target_ratio, max_delta);

                let (psola_l, psola_r) = self.dry.psola.process(
                    dry_l,
                    dry_r,
                    period,
                    pitch_ratio,
                    formant_follow,
                    force_unity,
                    latency,
                );
                self.dry.psola_path_gain += path_coeff * (1.0 - self.dry.psola_path_gain);
                let g = self.dry.psola_path_gain.clamp(0.0, 1.0);
                let wet_l = psola_l * g + delayed_l * (1.0 - g);
                let wet_r = psola_r * g + delayed_r * (1.0 - g);
                (wet_l, wet_r, target)
            } else {
                (delayed_l, delayed_r, 0.0)
            };

            // --- MIDI voices ---
            let mut sum_l = 0.0_f32;
            let mut sum_r = 0.0_f32;
            let mut active_count = 0_u32;
            let mut voice_midi = [0.0_f32; MAX_VOICES];
            let mut voice_target_hz = [0.0_f32; MAX_VOICES];

            for voice in self.voices.iter_mut() {
                if !voice.active {
                    continue;
                }
                let slot = active_count as usize;
                if slot < MAX_VOICES {
                    voice_midi[slot] = voice.midi_note as f32;
                }

                let target = voice.target.target_hz(&ctx);
                voice.last_target_hz = target;
                if slot < MAX_VOICES {
                    voice_target_hz[slot] = target;
                }

                Self::update_corrected_hz(
                    &mut voice.corrected_hz,
                    target,
                    sample_rate,
                    retune_ms,
                );

                // Exact MIDI Hz — no correction-range clamp (PSOLA hard-clamps).
                let target_ratio = if force_unity {
                    1.0
                } else if voice.corrected_hz > 0.0 && shift_f0 > 0.0 {
                    voice.corrected_hz / shift_f0
                } else {
                    1.0
                };
                let pitch_ratio =
                    Self::slew_ratio(&mut voice.applied_ratio, target_ratio, max_delta);

                let (psola_l, psola_r) = voice.psola.process(
                    dry_l,
                    dry_r,
                    period,
                    pitch_ratio,
                    formant_follow,
                    force_unity,
                    latency,
                );
                voice.psola_path_gain += path_coeff * (1.0 - voice.psola_path_gain);
                let g = voice.psola_path_gain.clamp(0.0, 1.0);
                sum_l += psola_l * g + delayed_l * (1.0 - g);
                sum_r += psola_r * g + delayed_r * (1.0 - g);
                active_count += 1;
            }

            let norm = if active_count > 0 {
                1.0 / active_count as f32
            } else {
                0.0
            };
            let voices_l = sum_l * norm * voices_level;
            let voices_r = sum_r * norm * voices_level;

            left[i] = under_l * dry_level + voices_l;
            right[i] = under_r * dry_level + voices_r;

            // Prefer smoothed LF contour for the visualizer; fall back to held f0.
            let meter_hz = if self.pitch_lf > 0.0 {
                self.pitch_lf
            } else {
                self.held_f0_hz
            };
            self.metering = PrismizerMetering {
                detected_hz: meter_hz,
                dry_target_hz,
                num_voices: active_count,
                voice_midi,
                voice_target_hz,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_matches_min_f0() {
        let mut params = PrismizerParams::default();
        params.min_f0_hz = 100.0;
        let node = Prismizer::new(48_000.0, &params);
        assert_eq!(node.reported_latency_samples(), 480);
    }

    #[test]
    fn note_on_off_updates_metering_voice_count() {
        let params = PrismizerParams::default();
        let mut node = Prismizer::new(48_000.0, &params);
        node.note_on(60, 1.0);
        node.note_on(64, 1.0);
        let mut left = vec![0.0_f32; 64];
        let mut right = vec![0.0_f32; 64];
        node.process_stereo(&mut left, &mut right);
        assert_eq!(node.metering().num_voices, 2);
        node.note_off(60);
        node.process_stereo(&mut left, &mut right);
        assert_eq!(node.metering().num_voices, 1);
    }

    #[test]
    fn steals_oldest_beyond_max_voices() {
        let params = PrismizerParams::default();
        let mut node = Prismizer::new(48_000.0, &params);
        for n in 0..MAX_VOICES as u8 {
            node.note_on(48 + n, 1.0);
        }
        node.note_on(80, 1.0);
        let mut left = vec![0.0_f32; 32];
        let mut right = vec![0.0_f32; 32];
        node.process_stereo(&mut left, &mut right);
        let m = node.metering();
        assert_eq!(m.num_voices, MAX_VOICES as u32);
        assert!(m.voice_midi[..MAX_VOICES].contains(&80.0));
        assert!(!m.voice_midi[..MAX_VOICES].contains(&48.0));
    }

    #[test]
    fn held_f0_keeps_shift_ready_across_short_silence() {
        let sr = 48_000.0_f32;
        let params = PrismizerParams {
            dry_correct: false,
            dry_level: 0.0,
            mix: 1.0,
            correction_amount: 1.0,
            vibrato_preserve: 0.0,
            retune_speed_ms: 1.0,
            min_f0_hz: 100.0,
            ..PrismizerParams::default()
        };
        let mut node = Prismizer::new(sr, &params);
        node.note_on(72, 1.0); // C5 ≈ 523 Hz from ~277 Hz input

        let f0 = 277.18_f32;
        let voiced = 12_000; // 250 ms
        let gap = 1_000; // ~21 ms silence — beyond YIN hold, within SHIFT_HOLD_MS
        let mut left = vec![0.0_f32; voiced + gap + 2_000];
        let mut right = left.clone();
        for i in 0..voiced {
            let s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin() * 0.5;
            left[i] = s;
            right[i] = s;
        }
        // gap stays zero
        for i in (voiced + gap)..left.len() {
            let t = i - voiced - gap;
            let s = (2.0 * std::f32::consts::PI * f0 * t as f32 / sr).sin() * 0.5;
            left[i] = s;
            right[i] = s;
        }

        // Process through voiced onset.
        node.process_stereo(&mut left[..voiced], &mut right[..voiced]);
        assert!(node.shift_ready, "should be shift-ready after sustained tone");
        assert!(node.held_f0_hz > 0.0);
        let held_before_gap = node.held_f0_hz;
        let ratio_before = node.voices.iter().find(|v| v.active).unwrap().applied_ratio;

        // Short silence: YIN may drop, but hold should keep shift_ready and ratios.
        node.process_stereo(
            &mut left[voiced..voiced + gap],
            &mut right[voiced..voiced + gap],
        );
        assert!(node.shift_ready, "shift_ready must survive short unvoiced gap");
        assert!(
            node.held_f0_hz > 0.0,
            "held_f0 must persist through short gap"
        );
        assert!((node.held_f0_hz - held_before_gap).abs() < 1.0);
        let ratio_during = node.voices.iter().find(|v| v.active).unwrap().applied_ratio;
        assert!(
            (ratio_during - ratio_before).abs() < 0.15,
            "MIDI ratio must not reset to unity across short gap (before={ratio_before}, during={ratio_during})"
        );
        assert!(
            node.metering().detected_hz > 0.0,
            "metering should stay non-zero during hold"
        );
    }
}
