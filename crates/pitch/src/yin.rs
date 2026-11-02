//! Causal YIN fundamental-frequency detection.

use crate::pitch_math::{hz_to_midi, one_pole_coeff};

/// Schema minimum `minF0Hz` — scratch sized for this floor (no audio-thread alloc).
/// ~50 Hz ≈ PDC latency of 20 ms @ 48 kHz; lower needs more buffer/CPU.
pub const SCHEMA_MIN_F0_HZ: f32 = 50.0;
/// Schema maximum `minF0Hz` (higher = lower latency, but ignores lower pitches).
pub const SCHEMA_MAX_F0_HZ: f32 = 200.0;
/// YIN window length in periods of `minF0Hz` (3 gives stabler low-F0 CMNDF).
const ANALYSIS_PERIODS: f32 = 3.0;
/// Baseline hop between YIN analyses (ms); lengthens automatically at low floors.
const ANALYSIS_HOP_MS: f32 = 5.0;
/// Cap hop so tracking still feels responsive when min F0 is very low.
const ANALYSIS_HOP_MAX_MS: f32 = 12.0;
/// Below this floor, run YIN on 2×-decimated audio (~4× less CPU).
const DOWNSAMPLE_BELOW_MIN_F0_HZ: f32 = 95.0;
/// YIN CMNDF threshold.
const YIN_THRESHOLD: f32 = 0.15;
/// Detection-path high-pass (Hz). Keep well below baritone fundamentals.
const INPUT_HP_HZ: f32 = 40.0;
/// Allow estimates slightly below `min_f0` (float / parabolic jitter).
const MIN_F0_REJECT_RATIO: f32 = 0.92;
/// Voiced confidence gate.
pub const VOICED_CONFIDENCE_MIN: f32 = 0.35;
/// Hold last F0 through brief unvoiced gaps (ms).
const UNVOICED_HOLD_MS: f32 = 15.0;
/// Reseat trackers when F0 jumps by more than this (semitones).
const F0_JUMP_RESET_SEMITONES: f32 = 3.0;
/// Median F0 history length.
pub const MAX_F0_HISTORY: usize = 5;
/// Upper detect limit.
const MAX_F0_HZ: f32 = 1200.0;

fn parabolic_interpolate_tau(cmndf: &[f32], tau: usize) -> f32 {
    if tau == 0 || tau + 1 >= cmndf.len() {
        return tau as f32;
    }
    let s0 = cmndf[tau - 1];
    let s1 = cmndf[tau];
    let s2 = cmndf[tau + 1];
    let denom = s0 - 2.0 * s1 + s2;
    if denom.abs() < 1.0e-12 {
        return tau as f32;
    }
    tau as f32 + 0.5 * (s0 - s2) / denom
}

/// YIN with caller-supplied F0 bounds. `diff` / `cmndf` must be ≥ max_tau+1.
pub fn yin_pitch_scratch(
    samples: &[f32],
    sample_rate: f32,
    min_f0_hz: f32,
    max_f0_hz: f32,
    diff: &mut [f32],
    cmndf: &mut [f32],
) -> (f32, f32) {
    if samples.len() < 64 {
        return (0.0, 0.0);
    }
    let min_f0 = min_f0_hz.max(1.0);
    let max_f0 = max_f0_hz.max(min_f0 + 1.0);
    let min_tau = ((sample_rate / max_f0).ceil() as usize).max(2);
    let max_tau = ((sample_rate / min_f0).floor() as usize).min(samples.len() / 2);
    if min_tau >= max_tau || diff.len() <= max_tau || cmndf.len() <= max_tau {
        return (0.0, 0.0);
    }

    let usable = samples.len() - max_tau;
    for tau in 1..=max_tau {
        let mut sum = 0.0_f32;
        for i in 0..usable {
            let d = samples[i] - samples[i + tau];
            sum += d * d;
        }
        diff[tau] = sum;
    }

    cmndf[0] = 1.0;
    let mut running = 0.0_f32;
    for tau in 1..=max_tau {
        running += diff[tau];
        cmndf[tau] = if running > 1.0e-12 {
            diff[tau] * tau as f32 / running
        } else {
            1.0
        };
    }

    let mut tau_est = min_tau;
    let mut found = false;
    for tau in min_tau..=max_tau {
        if cmndf[tau] < YIN_THRESHOLD {
            tau_est = tau;
            while tau_est + 1 <= max_tau && cmndf[tau_est + 1] < cmndf[tau_est] {
                tau_est += 1;
            }
            found = true;
            break;
        }
    }

    if !found {
        let mut min_val = f32::MAX;
        for tau in min_tau..=max_tau {
            if cmndf[tau] < min_val {
                min_val = cmndf[tau];
                tau_est = tau;
            }
        }
    }

    // Octave-error correction: first dip is often at T/2 when H2 dominates.
    // Walk to 2τ only when that trough is at least as deep (avoids 180→90 false folds).
    if found {
        while tau_est.saturating_mul(2) <= max_tau {
            let tau2 = tau_est * 2;
            if cmndf[tau2] >= YIN_THRESHOLD || cmndf[tau2] > cmndf[tau_est] {
                break;
            }
            let mut t = tau2;
            while t + 1 <= max_tau && cmndf[t + 1] < cmndf[t] {
                t += 1;
            }
            while t > min_tau && cmndf[t - 1] < cmndf[t] {
                t -= 1;
            }
            tau_est = t;
        }
    }

    let refined_tau = parabolic_interpolate_tau(cmndf, tau_est);
    if refined_tau <= 0.0 {
        return (0.0, 0.0);
    }

    let f0 = sample_rate / refined_tau;
    // Soft floor: parabolic refine can sit a hair below min_f0 at the search edge.
    if f0 < min_f0 * MIN_F0_REJECT_RATIO || f0 > max_f0 {
        return (0.0, 0.0);
    }

    let confidence = (1.0 - cmndf[tau_est.min(max_tau)]).clamp(0.0, 1.0);
    (f0, confidence)
}

fn median_smooth_f0(history: &mut [f32; MAX_F0_HISTORY], len: &mut usize, f0: f32) -> f32 {
    if *len < MAX_F0_HISTORY {
        history[*len] = f0;
        *len += 1;
    } else {
        history.copy_within(1.., 0);
        history[MAX_F0_HISTORY - 1] = f0;
    }
    let mut sorted: [f32; MAX_F0_HISTORY] = *history;
    let n = *len;
    sorted[..n].sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted[n / 2]
}

/// Stateful causal YIN detector with HP, hop, median, and unvoiced hold.
pub struct YinDetector {
    sample_rate: f32,
    hp_state: f32,
    hp_coeff: f32,
    analysis_buf: Vec<f32>,
    analysis_pos: usize,
    /// Nominal hop; refreshed from `min_f0` each analysis.
    hop_samples: usize,
    max_window_samples: usize,
    hop_counter: usize,
    pub detected_hz: f32,
    pub detected_confidence: f32,
    last_voiced_hz: f32,
    unvoiced_samples: usize,
    unvoiced_hold_samples: usize,
    pub f0_history: [f32; MAX_F0_HISTORY],
    pub f0_history_len: usize,
    yin_diff: Vec<f32>,
    yin_cmndf: Vec<f32>,
    analysis_ordered: Vec<f32>,
    /// 2×-decimated view of `analysis_ordered` for low-floor YIN.
    analysis_decimated: Vec<f32>,
    /// Set when F0 jump should reseat LF/HF trackers in the corrector.
    pub jump_reseat: bool,
}

impl YinDetector {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let max_period = ((sr / SCHEMA_MIN_F0_HZ).ceil() as usize).max(64);
        let max_window = ((ANALYSIS_PERIODS * max_period as f32).ceil() as usize).max(128);
        let hop_samples = Self::hop_samples_for(sr, SCHEMA_MIN_F0_HZ);
        // Diff buffers sized for full-rate max period (decimated uses half).
        let yin_cap = max_period + 8;
        Self {
            sample_rate: sr,
            hp_state: 0.0,
            hp_coeff: one_pole_coeff(sr, 1000.0 / (2.0 * std::f32::consts::PI * INPUT_HP_HZ)),
            analysis_buf: vec![0.0; max_window],
            analysis_pos: 0,
            hop_samples,
            max_window_samples: max_window,
            hop_counter: 0,
            detected_hz: 0.0,
            detected_confidence: 0.0,
            last_voiced_hz: 0.0,
            unvoiced_samples: 0,
            unvoiced_hold_samples: ((UNVOICED_HOLD_MS / 1000.0) * sr).round() as usize,
            f0_history: [0.0; MAX_F0_HISTORY],
            f0_history_len: 0,
            yin_diff: vec![0.0; yin_cap],
            yin_cmndf: vec![1.0; yin_cap],
            analysis_ordered: vec![0.0; max_window],
            analysis_decimated: vec![0.0; (max_window / 2) + 8],
            jump_reseat: false,
        }
    }

    fn hop_samples_for(sample_rate: f32, min_f0: f32) -> usize {
        let min_f0 = min_f0.max(SCHEMA_MIN_F0_HZ);
        // ~0.4 of one floor-period, clamped — fewer huge YIN jobs at bass floors.
        let period_ms = 1000.0 / min_f0;
        let hop_ms = (period_ms * 0.4).clamp(ANALYSIS_HOP_MS, ANALYSIS_HOP_MAX_MS);
        ((hop_ms / 1000.0) * sample_rate).round().max(32.0) as usize
    }

    pub fn reset(&mut self) {
        self.hp_state = 0.0;
        self.analysis_buf.fill(0.0);
        self.analysis_pos = 0;
        self.hop_counter = 0;
        self.clear_pitch_state();
    }

    pub fn clear_pitch_state(&mut self) {
        self.detected_hz = 0.0;
        self.detected_confidence = 0.0;
        self.last_voiced_hz = 0.0;
        self.f0_history_len = 0;
        self.unvoiced_samples = 0;
        self.jump_reseat = false;
    }

    fn window_samples_for_min_f0(&self, min_f0: f32) -> usize {
        let period = (self.sample_rate / min_f0.max(SCHEMA_MIN_F0_HZ)).ceil() as usize;
        ((ANALYSIS_PERIODS * period as f32).ceil() as usize)
            .max(64)
            .min(self.max_window_samples)
    }

    fn push_analysis_sample(&mut self, sample: f32) {
        if self.analysis_buf.is_empty() {
            return;
        }
        self.analysis_buf[self.analysis_pos] = sample;
        self.analysis_pos = (self.analysis_pos + 1) % self.max_window_samples;
    }

    fn fill_ordered_analysis_window(&mut self, window_samples: usize) {
        let n = window_samples.min(self.max_window_samples);
        for i in 0..n {
            let idx = (self.analysis_pos + self.max_window_samples - n + i) % self.max_window_samples;
            self.analysis_ordered[i] = self.analysis_buf[idx];
        }
    }

    fn run_analysis(&mut self, min_f0: f32) {
        let min_f0 = min_f0.max(SCHEMA_MIN_F0_HZ);
        self.hop_samples = Self::hop_samples_for(self.sample_rate, min_f0);

        let window = self.window_samples_for_min_f0(min_f0);
        self.fill_ordered_analysis_window(window);

        let use_decimate = min_f0 <= DOWNSAMPLE_BELOW_MIN_F0_HZ && window >= 128;
        let (f0, confidence) = if use_decimate {
            let n_dec = window / 2;
            for i in 0..n_dec {
                // Box-ish 2-tap average = cheap anti-alias before decimation.
                let a = self.analysis_ordered[i * 2];
                let b = self.analysis_ordered[i * 2 + 1];
                self.analysis_decimated[i] = 0.5 * (a + b);
            }
            yin_pitch_scratch(
                &self.analysis_decimated[..n_dec],
                self.sample_rate * 0.5,
                min_f0,
                MAX_F0_HZ,
                &mut self.yin_diff,
                &mut self.yin_cmndf,
            )
        } else {
            yin_pitch_scratch(
                &self.analysis_ordered[..window],
                self.sample_rate,
                min_f0,
                MAX_F0_HZ,
                &mut self.yin_diff,
                &mut self.yin_cmndf,
            )
        };
        self.detected_confidence = confidence;
        self.jump_reseat = false;

        if confidence >= VOICED_CONFIDENCE_MIN && f0 > 0.0 {
            self.unvoiced_samples = 0;
            if self.detected_hz > 0.0 {
                let jump =
                    (hz_to_midi(f0, 440.0) - hz_to_midi(self.detected_hz, 440.0)).abs();
                if jump >= F0_JUMP_RESET_SEMITONES {
                    self.jump_reseat = true;
                }
            } else {
                self.jump_reseat = true;
            }
            self.detected_hz =
                median_smooth_f0(&mut self.f0_history, &mut self.f0_history_len, f0);
            self.last_voiced_hz = self.detected_hz;
            return;
        }

        self.unvoiced_samples = self.unvoiced_samples.saturating_add(self.hop_samples);
        if self.unvoiced_samples < self.unvoiced_hold_samples && self.last_voiced_hz > 0.0 {
            self.detected_hz = self.last_voiced_hz;
        } else if self.last_voiced_hz > 0.0 {
            self.clear_pitch_state();
        }
    }

    /// Push one mono sample (pre-sum). Returns true if a hop analysis ran.
    pub fn process_sample(&mut self, mono: f32, min_f0: f32, detect_enabled: bool) {
        self.hp_state += self.hp_coeff * (mono - self.hp_state);
        let hp = mono - self.hp_state;
        self.push_analysis_sample(hp);

        self.hop_counter += 1;
        if self.hop_counter >= self.hop_samples {
            self.hop_counter = 0;
            if detect_enabled {
                self.run_analysis(min_f0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive_detector(f0: f32, min_f0: f32, seconds: f32, fund_amp: f32) -> (f32, f32) {
        let sr = 48_000.0_f32;
        let mut det = YinDetector::new(sr);
        let n = (sr * seconds) as usize;
        for i in 0..n {
            // Voice-like: optional weak fundamental + stronger lower harmonics.
            let t = i as f32 / sr;
            let s = (2.0 * std::f32::consts::PI * f0 * t).sin() * fund_amp
                + (2.0 * std::f32::consts::PI * f0 * 2.0 * t).sin() * 0.55
                + (2.0 * std::f32::consts::PI * f0 * 3.0 * t).sin() * 0.35
                + (2.0 * std::f32::consts::PI * f0 * 4.0 * t).sin() * 0.2;
            det.process_sample(s, min_f0, true);
        }
        (det.detected_hz, det.detected_confidence)
    }

    #[test]
    fn yin_respects_raised_min_f0() {
        let sr = 48_000.0_f32;
        let f0 = 90.0_f32;
        let period = (sr / f0).round() as usize;
        let n = period * 4;
        let mut samples = vec![0.0_f32; n];
        for (i, s) in samples.iter_mut().enumerate() {
            *s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin();
        }
        let max_tau = ((sr / SCHEMA_MIN_F0_HZ).floor() as usize) + 8;
        let mut diff = vec![0.0; max_tau];
        let mut cmndf = vec![1.0; max_tau];
        let (detected, conf) =
            yin_pitch_scratch(&samples, sr, 80.0, MAX_F0_HZ, &mut diff, &mut cmndf);
        assert!(conf > 0.2, "confidence {conf}");
        assert!((detected - f0).abs() < 5.0, "detected {detected} vs {f0}");
        let (raised, _) =
            yin_pitch_scratch(&samples, sr, 150.0, MAX_F0_HZ, &mut diff, &mut cmndf);
        assert!(
            raised < 1.0 || (raised - f0).abs() > 20.0,
            "raised min should not lock the true F0; got {raised}"
        );
    }

    #[test]
    fn detector_locks_below_200_hz() {
        for f0 in [110.0_f32, 130.0, 150.0, 180.0] {
            let (hz, conf) = drive_detector(f0, 100.0, 0.35, 0.6);
            assert!(
                hz > 0.0 && (hz - f0).abs() < 8.0,
                "failed to lock {f0} Hz: got {hz} (conf {conf})"
            );
        }
    }

    #[test]
    fn detector_locks_near_schema_floor() {
        for f0 in [55.0_f32, 65.0, 80.0] {
            let (hz, conf) = drive_detector(f0, SCHEMA_MIN_F0_HZ, 0.5, 0.55);
            assert!(
                hz > 0.0 && (hz - f0).abs() < 6.0,
                "failed to lock {f0} Hz at schema floor: got {hz} (conf {conf})"
            );
        }
    }

    #[test]
    fn detector_locks_weak_fundamental_below_200_hz() {
        // Typical sung note: strong H2/H3, weaker F0 — YIN often octave-errors here.
        for f0 in [110.0_f32, 130.0, 150.0, 180.0] {
            let (hz, conf) = drive_detector(f0, 80.0, 0.45, 0.15);
            assert!(
                hz > 0.0 && (hz - f0).abs() < 10.0,
                "weak-F0 failed to lock {f0} Hz: got {hz} (conf {conf})"
            );
        }
    }

    #[test]
    fn detector_locks_missing_fundamental_below_200_hz() {
        for f0 in [110.0_f32, 130.0, 150.0, 180.0] {
            let (hz, conf) = drive_detector(f0, 80.0, 0.5, 0.0);
            assert!(
                hz > 0.0 && (hz - f0).abs() < 12.0,
                "missing-F0 failed to lock {f0} Hz: got {hz} (conf {conf})"
            );
        }
    }
}
