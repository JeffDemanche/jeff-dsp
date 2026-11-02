//! Composed monophonic pitch corrector (detect → target → retune → PSOLA → mix).

use crate::delay_line::DelayLine;
use crate::pitch_math::{clamp_ratio_to_correction_range, one_pole_coeff};
use crate::psola::PsolaShifter;
use crate::smoothed::SmoothedValue;
use crate::target::{PitchTarget, ScalePitchTarget, TargetContext};
use crate::yin::{YinDetector, SCHEMA_MAX_F0_HZ, SCHEMA_MIN_F0_HZ};
use crate::{DEFAULT_MIN_F0_HZ, PARAM_SMOOTH_MS};

/// Slow pitch contour for intended-note quantization (ms).
const PITCH_LF_SMOOTH_MS: f32 = 15.0;
/// Fast pitch contour for vibrato extraction (ms).
const PITCH_HF_SMOOTH_MS: f32 = 6.0;
/// Default quantize dead zone (cents).
const DEFAULT_QUANTIZE_DEAD_ZONE_CENTS: f32 = 8.0;
/// Voiced F0 history samples before non-unity shift.
const MIN_F0_HISTORY_FOR_SHIFT: usize = 2;
/// Max pitch-ratio change per millisecond while retuning.
const MAX_RATIO_SLEW_PER_MS: f32 = 0.04;
/// Crossfade wet (PSOLA) vs delayed-dry when correction engages (ms).
const PSOLA_PATH_FADE_MS: f32 = 4.0;

/// User-facing parameters (aligned with Vamp LL).
#[derive(Debug, Clone)]
pub struct CorrectorParams {
    pub key_semitone: u8,
    pub scale: u8,
    pub reference_hz: f32,
    pub retune_speed_ms: f32,
    pub correction_amount: f32,
    pub correction_range_semitones: f32,
    pub vibrato_preserve: f32,
    pub formant_follow: f32,
    pub quantize_dead_zone_cents: f32,
    pub mix: f32,
    pub min_f0_hz: f32,
}

impl Default for CorrectorParams {
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
        }
    }
}

/// Low-latency monophonic pitch corrector parameterized by a [`PitchTarget`].
pub struct Corrector<T: PitchTarget = ScalePitchTarget> {
    sample_rate: f32,
    target: T,
    reference_hz: SmoothedValue,
    retune_speed_ms: SmoothedValue,
    correction_amount: SmoothedValue,
    correction_range: SmoothedValue,
    vibrato_preserve: SmoothedValue,
    formant_follow: SmoothedValue,
    quantize_dead_zone_cents: SmoothedValue,
    mix: SmoothedValue,
    min_f0_hz: SmoothedValue,
    detector: YinDetector,
    pitch_lf: f32,
    pitch_hf: f32,
    pitch_lf_coeff: f32,
    pitch_hf_coeff: f32,
    corrected_hz: f32,
    last_target_hz: f32,
    last_ratio: f32,
    applied_ratio: f32,
    max_ratio_delta_per_sample: f32,
    psola_path_gain: f32,
    psola_path_coeff: f32,
    psola: PsolaShifter,
    dry_delay_l: DelayLine,
    dry_delay_r: DelayLine,
}

impl Corrector<ScalePitchTarget> {
    pub fn new_scale(sample_rate: f32, params: &CorrectorParams) -> Self {
        Self::new(sample_rate, params, ScalePitchTarget::new(params.key_semitone, params.scale))
    }
}

impl<T: PitchTarget> Corrector<T> {
    pub fn new(sample_rate: f32, params: &CorrectorParams, target: T) -> Self {
        let sr = sample_rate.max(1.0);
        let dry_cap = ((sr / SCHEMA_MIN_F0_HZ).ceil() as usize) + 16;

        Self {
            sample_rate: sr,
            target,
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
            detector: YinDetector::new(sr),
            pitch_lf: 0.0,
            pitch_hf: 0.0,
            pitch_lf_coeff: one_pole_coeff(sr, PITCH_LF_SMOOTH_MS),
            pitch_hf_coeff: one_pole_coeff(sr, PITCH_HF_SMOOTH_MS),
            corrected_hz: 0.0,
            last_target_hz: 0.0,
            last_ratio: 1.0,
            applied_ratio: 1.0,
            max_ratio_delta_per_sample: MAX_RATIO_SLEW_PER_MS / (sr / 1000.0).max(1.0),
            psola_path_gain: 0.0,
            psola_path_coeff: one_pole_coeff(sr, PSOLA_PATH_FADE_MS),
            psola: PsolaShifter::new(sr),
            dry_delay_l: DelayLine::new(dry_cap),
            dry_delay_r: DelayLine::new(dry_cap),
        }
    }

    pub fn target_mut(&mut self) -> &mut T {
        &mut self.target
    }

    /// Algorithmic output latency for PDC (≈ one period at `minF0Hz`).
    pub fn reported_latency_samples(&self) -> u32 {
        let min_f0 = self.min_f0_hz.value().clamp(SCHEMA_MIN_F0_HZ, SCHEMA_MAX_F0_HZ);
        (self.sample_rate / min_f0).round().max(1.0) as u32
    }

    pub fn set_params(&mut self, params: &CorrectorParams) {
        self.reference_hz.set_target(params.reference_hz.clamp(400.0, 480.0));
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
    }

    pub fn reset(&mut self) {
        self.detector.reset();
        self.target.reset();
        self.pitch_lf = 0.0;
        self.pitch_hf = 0.0;
        self.corrected_hz = 0.0;
        self.last_target_hz = 0.0;
        self.last_ratio = 1.0;
        self.applied_ratio = 1.0;
        self.psola_path_gain = 0.0;
        self.psola.reset();
        self.dry_delay_l.clear();
        self.dry_delay_r.clear();
    }

    fn clear_pitch_trackers(&mut self) {
        self.pitch_lf = 0.0;
        self.pitch_hf = 0.0;
        self.corrected_hz = 0.0;
        self.last_target_hz = 0.0;
        self.last_ratio = 1.0;
        self.applied_ratio = 1.0;
        self.target.reset();
    }

    fn reseat_on_f0_jump(&mut self, new_hz: f32) {
        self.pitch_lf = new_hz;
        self.pitch_hf = new_hz;
        self.corrected_hz = 0.0;
        self.target.reset();
    }

    fn update_pitch_split(&mut self) {
        if self.detector.detected_hz <= 0.0 {
            return;
        }
        if self.pitch_lf <= 0.0 {
            self.pitch_lf = self.detector.detected_hz;
            self.pitch_hf = self.detector.detected_hz;
            return;
        }
        self.pitch_lf += self.pitch_lf_coeff * (self.detector.detected_hz - self.pitch_lf);
        self.pitch_hf += self.pitch_hf_coeff * (self.detector.detected_hz - self.pitch_hf);
    }

    fn update_corrected_hz(&mut self, target: f32, retune_ms: f32) {
        if target <= 0.0 {
            return;
        }
        if self.corrected_hz <= 0.0 {
            self.corrected_hz = target;
            return;
        }
        let coeff = one_pole_coeff(self.sample_rate, retune_ms.max(0.5));
        self.corrected_hz += coeff * (target - self.corrected_hz);
    }

    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        for i in 0..frames {
            let reference = self.reference_hz.next();
            let retune_ms = self.retune_speed_ms.next();
            let amount = self.correction_amount.next();
            let range = self.correction_range.next();
            let vibrato_keep = self.vibrato_preserve.next();
            let formant_follow = self.formant_follow.next().clamp(0.0, 1.0);
            let dead_zone_cents = self.quantize_dead_zone_cents.next().clamp(0.0, 50.0);
            let mix = self.mix.next().clamp(0.0, 1.0);
            let min_f0 = self
                .min_f0_hz
                .next()
                .clamp(SCHEMA_MIN_F0_HZ, SCHEMA_MAX_F0_HZ);
            let latency = (self.sample_rate / min_f0).max(1.0);

            let dry_l = left[i];
            let dry_r = right[i];

            let delayed_l = self.dry_delay_l.read_at(latency);
            let delayed_r = self.dry_delay_r.read_at(latency);
            self.dry_delay_l.write_sample(dry_l);
            self.dry_delay_r.write_sample(dry_r);

            let mono = (dry_l + dry_r) * 0.5;
            let prev_hz = self.detector.detected_hz;
            self.detector.process_sample(mono, min_f0, true);
            if self.detector.jump_reseat {
                self.reseat_on_f0_jump(self.detector.detected_hz);
            } else if prev_hz > 0.0 && self.detector.detected_hz <= 0.0 {
                self.clear_pitch_trackers();
            }

            self.update_pitch_split();

            let ctx = TargetContext {
                detected_hz: self.detector.detected_hz,
                pitch_lf: self.pitch_lf,
                pitch_hf: self.pitch_hf,
                reference_hz: reference,
                correction_amount: amount,
                correction_range_semitones: range,
                vibrato_preserve: vibrato_keep,
                quantize_dead_zone_cents: dead_zone_cents,
                quantize_enabled: true,
            };
            let target = self.target.target_hz(&ctx);
            self.last_target_hz = target;
            self.update_corrected_hz(target, retune_ms);

            let target_ratio = if self.detector.f0_history_len < MIN_F0_HISTORY_FOR_SHIFT {
                1.0
            } else if self.corrected_hz > 0.0 && self.detector.detected_hz > 0.0 {
                clamp_ratio_to_correction_range(
                    self.corrected_hz / self.detector.detected_hz,
                    range,
                )
            } else {
                1.0
            };

            let max_delta = self.max_ratio_delta_per_sample;
            let delta = (target_ratio - self.applied_ratio).clamp(-max_delta, max_delta);
            self.applied_ratio += delta;
            let pitch_ratio = self.applied_ratio;
            self.last_ratio = pitch_ratio;

            let period = if self.detector.detected_hz > 0.0 {
                self.sample_rate / self.detector.detected_hz
            } else {
                latency
            };

            let correcting = self.detector.detected_hz > 0.0
                && self.detector.f0_history_len >= MIN_F0_HISTORY_FOR_SHIFT;
            let force_unity = !correcting;

            let (psola_l, psola_r) = self.psola.process(
                dry_l,
                dry_r,
                period,
                pitch_ratio,
                formant_follow,
                force_unity,
                latency,
            );

            self.psola_path_gain += self.psola_path_coeff * (1.0 - self.psola_path_gain);
            let g = self.psola_path_gain.clamp(0.0, 1.0);
            let wet_l = psola_l * g + delayed_l * (1.0 - g);
            let wet_r = psola_r * g + delayed_r * (1.0 - g);

            let dry_amt = 1.0 - mix;
            left[i] = delayed_l * dry_amt + wet_l * mix;
            right[i] = delayed_r * dry_amt + wet_r * mix;
        }
    }
}

/// Scale-target helpers used by FFI when setting key/scale.
impl Corrector<ScalePitchTarget> {
    pub fn set_key_scale(&mut self, key_semitone: u8, scale: u8) {
        self.target.set_key_scale(key_semitone, scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latency_scales_with_min_f0() {
        let mut params = CorrectorParams::default();
        params.min_f0_hz = 50.0;
        let node_lo = Corrector::new_scale(48_000.0, &params);
        params.min_f0_hz = 200.0;
        let node_hi = Corrector::new_scale(48_000.0, &params);
        assert_eq!(node_lo.reported_latency_samples(), 960);
        assert_eq!(node_hi.reported_latency_samples(), 240);
    }

    #[test]
    fn psola_pitch_shift_not_silent() {
        let sr = 48_000.0;
        let params = CorrectorParams {
            scale: 0,
            correction_amount: 1.0,
            correction_range_semitones: 4.0,
            retune_speed_ms: 0.0,
            vibrato_preserve: 0.0,
            mix: 1.0,
            min_f0_hz: 100.0,
            ..CorrectorParams::default()
        };
        let mut node = Corrector::new_scale(sr, &params);
        let f0 = 277.0_f32;
        let frames = 4096;
        let mut left = vec![0.0_f32; frames];
        let mut right = vec![0.0_f32; frames];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin() * 0.5;
            left[i] = s;
            right[i] = s;
        }
        node.process_stereo(&mut left, &mut right);
        let peak = left.iter().map(|v| v.abs()).fold(0.0_f32, f32::max);
        assert!(peak > 0.05, "peak {peak} — shifter should pass energy");
    }

    fn rough_f0(buf: &[f32], sr: f32, min_hz: f32, max_hz: f32) -> f32 {
        let min_tau = (sr / max_hz).ceil() as usize;
        let max_tau = (sr / min_hz).floor() as usize;
        let mut best_tau = min_tau;
        let mut best = f32::MAX;
        for tau in min_tau..=max_tau.min(buf.len() / 2) {
            let mut d = 0.0_f32;
            for i in 0..(buf.len() - tau) {
                let e = buf[i] - buf[i + tau];
                d += e * e;
            }
            if d < best {
                best = d;
                best_tau = tau;
            }
        }
        sr / best_tau as f32
    }

    #[test]
    fn major_scale_snaps_csharp_toward_c_or_d() {
        let sr = 48_000.0;
        let params = CorrectorParams {
            key_semitone: 0,
            scale: 1, // Major
            correction_amount: 1.0,
            correction_range_semitones: 2.0,
            retune_speed_ms: 1.0,
            vibrato_preserve: 0.0,
            formant_follow: 0.0,
            quantize_dead_zone_cents: 0.0,
            mix: 1.0,
            min_f0_hz: 100.0,
            reference_hz: 440.0,
        };
        let mut node = Corrector::new_scale(sr, &params);
        let f0_in = 277.18_f32; // C#4
        let frames = 24_000; // 0.5 s
        let mut left = vec![0.0_f32; frames];
        let mut right = vec![0.0_f32; frames];
        for i in 0..frames {
            let s = (2.0 * std::f32::consts::PI * f0_in * i as f32 / sr).sin() * 0.5;
            left[i] = s;
            right[i] = s;
        }
        node.process_stereo(&mut left, &mut right);
        let f0_out = rough_f0(&left[frames - 8192..], sr, 180.0, 380.0);
        let c4 = 261.63_f32;
        let d4 = 293.66_f32;
        let toward_c = (f0_out - c4).abs() < (f0_in - c4).abs() - 5.0;
        let toward_d = (f0_out - d4).abs() < (f0_in - d4).abs() - 5.0;
        assert!(
            toward_c || toward_d,
            "expected snap away from C#4={f0_in:.1}, got {f0_out:.1}"
        );
    }
}
