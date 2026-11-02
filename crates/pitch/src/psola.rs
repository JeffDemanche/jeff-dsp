//! Stereo TD-PSOLA pitch shifter with optional formant follow.

use crate::yin::SCHEMA_MIN_F0_HZ;
use crate::DEFAULT_MIN_F0_HZ;

/// Treat |ratio−1| below this as unity.
const UNITY_RATIO_EPS: f32 = 0.0005;

fn hann(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
}

/// Stereo TD-PSOLA with shared pitch marks / ratio.
///
/// Duration-preserving pitch shift: analysis marks advance every `pa` input
/// samples; synthesis grains are placed every `pa/ratio` output samples and
/// read from the latest analysis mark (reused when raising pitch, skipped when
/// lowering). Always keeps the OLA path running (unity-ratio grains when not
/// correcting) so voiced↔unvoiced transitions do not hard-switch wet/dry.
pub struct PsolaShifter {
    capacity: usize,
    in_l: Vec<f32>,
    in_r: Vec<f32>,
    write: usize,
    ola_l: Vec<f32>,
    ola_r: Vec<f32>,
    ola_read: usize,
    synth_countdown: f32,
    analysis_countdown: f32,
    /// Delay (samples behind write) of the latest analysis mark.
    mark_delay: f32,
    filled: usize,
    /// Last stable analysis period (samples); held across unvoiced gaps.
    last_period: f32,
}

impl PsolaShifter {
    pub fn new(sample_rate: f32) -> Self {
        let max_period = ((sample_rate / SCHEMA_MIN_F0_HZ).ceil() as usize).max(64);
        let capacity = (max_period * 8).max(2048);
        let default_period = (sample_rate / DEFAULT_MIN_F0_HZ).max(2.0);
        Self {
            capacity,
            in_l: vec![0.0; capacity],
            in_r: vec![0.0; capacity],
            write: 0,
            ola_l: vec![0.0; capacity],
            ola_r: vec![0.0; capacity],
            ola_read: 0,
            synth_countdown: 0.0,
            analysis_countdown: 0.0,
            mark_delay: default_period,
            filled: 0,
            last_period: default_period,
        }
    }

    pub fn reset(&mut self) {
        self.in_l.fill(0.0);
        self.in_r.fill(0.0);
        self.ola_l.fill(0.0);
        self.ola_r.fill(0.0);
        self.write = 0;
        self.ola_read = 0;
        self.synth_countdown = 0.0;
        self.analysis_countdown = 0.0;
        self.mark_delay = self.last_period;
        self.filled = 0;
    }

    fn read_input(&self, delay_samples: f32) -> (f32, f32) {
        let len = self.capacity as f32;
        let delay = delay_samples.clamp(1.0, len - 1.0);
        let read_pos = self.write as f32 - delay;
        let wrapped = if read_pos >= 0.0 {
            read_pos
        } else {
            read_pos + len * ((-read_pos / len).floor() + 1.0)
        };
        let index = wrapped % len;
        let i0 = index.floor() as usize % self.capacity;
        let i1 = (i0 + 1) % self.capacity;
        let frac = index - index.floor();
        let l = self.in_l[i0] * (1.0 - frac) + self.in_l[i1] * frac;
        let r = self.in_r[i0] * (1.0 - frac) + self.in_r[i1] * frac;
        (l, r)
    }

    /// Place a Hann grain centered on `mark_delay` (samples behind write).
    /// `formant_follow` 0 keeps grain samples as-is (preserve); 1 resamples
    /// around the mark by `ratio` so formants move with pitch.
    fn place_grain(&mut self, mark_delay: f32, period_samples: f32, ratio: f32, formant_follow: f32) {
        let pa = period_samples.max(2.0);
        let grain_len = ((2.0 * pa).round() as usize).max(4).min(self.capacity / 2);
        let follow = formant_follow.clamp(0.0, 1.0);
        let step = 1.0 + (ratio - 1.0) * follow;
        let half = grain_len as f32 * 0.5;
        let max_delay = (self.capacity as f32) - 1.0;
        for i in 0..grain_len {
            let w = hann(i, grain_len);
            // i=0 → older (larger delay); i=grain_len-1 → newer (smaller delay).
            let src_delay = (mark_delay - (i as f32 - half) * step).clamp(1.0, max_delay);
            let (sl, sr) = self.read_input(src_delay);
            let idx = (self.ola_read + i) % self.capacity;
            self.ola_l[idx] += sl * w;
            self.ola_r[idx] += sr * w;
        }
    }

    /// Process one stereo frame.
    pub fn process(
        &mut self,
        left: f32,
        right: f32,
        period_samples: f32,
        ratio: f32,
        formant_follow: f32,
        force_unity: bool,
        latency_samples: f32,
    ) -> (f32, f32) {
        self.in_l[self.write] = left;
        self.in_r[self.write] = right;
        self.write = (self.write + 1) % self.capacity;
        self.filled = (self.filled + 1).min(self.capacity);

        // Marks age by one sample as the write head advances.
        self.mark_delay = (self.mark_delay + 1.0).min((self.capacity as f32) - 1.0);

        if period_samples.is_finite() && period_samples > 2.0 {
            self.last_period = period_samples;
        }
        let pa = self.last_period.max(2.0);
        let min_fill = ((3.0 * pa).ceil() as usize).saturating_add(8);

        // Temporary first-pass range (±2 octaves). Octave fold / formant
        // improvements for larger MIDI shifts are deferred.
        let ratio = if force_unity || (ratio - 1.0).abs() < UNITY_RATIO_EPS {
            1.0
        } else {
            ratio.clamp(0.25, 4.0)
        };
        let follow = if force_unity { 0.0 } else { formant_follow };

        // Analysis marks every analysis period. Seed the mark one period behind
        // the write head so a 2-period grain fits in history.
        self.analysis_countdown -= 1.0;
        while self.analysis_countdown <= 0.0 {
            self.analysis_countdown += pa;
            self.mark_delay = pa;
        }

        if self.filled >= min_fill {
            let ps = (pa / ratio).max(2.0);
            self.synth_countdown -= 1.0;
            while self.synth_countdown <= 0.0 {
                self.place_grain(self.mark_delay, pa, ratio, follow);
                self.synth_countdown += ps;
            }
            let out_l = self.ola_l[self.ola_read];
            let out_r = self.ola_r[self.ola_read];
            self.ola_l[self.ola_read] = 0.0;
            self.ola_r[self.ola_read] = 0.0;
            self.ola_read = (self.ola_read + 1) % self.capacity;
            (out_l, out_r)
        } else {
            let delay = latency_samples.clamp(1.0, (self.capacity as f32) - 1.0);
            if self.filled as f32 >= delay {
                self.read_input(delay)
            } else {
                (0.0, 0.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ratio_raises_pitch() {
        let sr = 48_000.0_f32;
        let mut psola = PsolaShifter::new(sr);
        let f0 = 277.18_f32;
        let period = sr / f0;
        let ratio = 1.0595_f32; // +1 st
        let latency = sr / 100.0;
        let n = 24_000;
        let mut out = vec![0.0_f32; n];
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin() * 0.5;
            let (l, _) = psola.process(s, s, period, ratio, 0.0, false, latency);
            out[i] = l;
        }
        let f0_out = rough_f0(&out[n - 8192..], sr, 180.0, 380.0);
        assert!(
            f0_out > f0 + 8.0,
            "expected raised pitch, got {f0_out} from {f0}"
        );
    }

    #[test]
    fn ratio_lowers_pitch() {
        let sr = 48_000.0_f32;
        let mut psola = PsolaShifter::new(sr);
        let f0 = 277.18_f32;
        let period = sr / f0;
        let ratio = 1.0 / 1.0595_f32; // -1 st
        let latency = sr / 100.0;
        let n = 24_000;
        let mut out = vec![0.0_f32; n];
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin() * 0.5;
            let (l, _) = psola.process(s, s, period, ratio, 0.0, false, latency);
            out[i] = l;
        }
        let f0_out = rough_f0(&out[n - 8192..], sr, 180.0, 380.0);
        assert!(
            f0_out < f0 - 8.0,
            "expected lowered pitch, got {f0_out} from {f0}"
        );
    }

    #[test]
    fn unity_keeps_pitch() {
        let sr = 48_000.0_f32;
        let mut psola = PsolaShifter::new(sr);
        let f0 = 277.18_f32;
        let period = sr / f0;
        let latency = sr / 100.0;
        let n = 24_000;
        let mut out = vec![0.0_f32; n];
        for i in 0..n {
            let s = (2.0 * std::f32::consts::PI * f0 * i as f32 / sr).sin() * 0.5;
            let (l, _) = psola.process(s, s, period, 1.0, 0.0, true, latency);
            out[i] = l;
        }
        let f0_out = rough_f0(&out[n - 8192..], sr, 180.0, 380.0);
        assert!((f0_out - f0).abs() < 8.0, "unity drifted: {f0_out}");
    }
}
