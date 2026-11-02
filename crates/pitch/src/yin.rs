//! Causal YIN fundamental-frequency detection.

use crate::pitch_math::{hz_to_midi, one_pole_coeff};

/// Schema minimum `minF0Hz` — scratch sized for this floor (no audio-thread alloc).
pub const SCHEMA_MIN_F0_HZ: f32 = 80.0;
/// Schema maximum `minF0Hz`.
pub const SCHEMA_MAX_F0_HZ: f32 = 200.0;
/// YIN window length in periods of `minF0Hz`.
const ANALYSIS_PERIODS: f32 = 2.0;
/// Hop between YIN analyses (ms).
const ANALYSIS_HOP_MS: f32 = 5.0;
/// YIN CMNDF threshold.
const YIN_THRESHOLD: f32 = 0.15;
/// Detection-path high-pass (Hz).
const INPUT_HP_HZ: f32 = 80.0;
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

    let refined_tau = parabolic_interpolate_tau(cmndf, tau_est);
    if refined_tau <= 0.0 {
        return (0.0, 0.0);
    }

    let f0 = sample_rate / refined_tau;
    if f0 < min_f0 || f0 > max_f0 {
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
    /// Set when F0 jump should reseat LF/HF trackers in the corrector.
    pub jump_reseat: bool,
}

impl YinDetector {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let max_period = ((sr / SCHEMA_MIN_F0_HZ).ceil() as usize).max(64);
        let max_window = ((ANALYSIS_PERIODS * max_period as f32).ceil() as usize).max(128);
        let hop_samples = (((ANALYSIS_HOP_MS / 1000.0) * sr).round() as usize).max(32);
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
            jump_reseat: false,
        }
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
        let window = self.window_samples_for_min_f0(min_f0);
        self.fill_ordered_analysis_window(window);
        let (f0, confidence) = yin_pitch_scratch(
            &self.analysis_ordered[..window],
            self.sample_rate,
            min_f0,
            MAX_F0_HZ,
            &mut self.yin_diff,
            &mut self.yin_cmndf,
        );
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
}
