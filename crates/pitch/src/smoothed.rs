//! One-pole exponential smoothing for parameter changes.

pub struct SmoothedValue {
    current: f32,
    target: f32,
    coeff: f32,
}

impl SmoothedValue {
    pub fn new(initial: f32, sample_rate: f32, time_ms: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            coeff: Self::coeff_for(sample_rate, time_ms),
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
    }

    pub fn reset_sample_rate(&mut self, sample_rate: f32, time_ms: f32) {
        self.coeff = Self::coeff_for(sample_rate, time_ms);
    }

    pub fn next(&mut self) -> f32 {
        self.advance(1);
        self.current
    }

    pub fn advance(&mut self, samples: usize) {
        if samples == 0 {
            return;
        }
        if (self.current - self.target).abs() < 1.0e-6 {
            self.current = self.target;
            return;
        }
        let remain = (1.0 - self.coeff).powi(samples as i32);
        self.current = self.target + (self.current - self.target) * remain;
        if (self.current - self.target).abs() < 1.0e-6 {
            self.current = self.target;
        }
    }

    pub fn value(&self) -> f32 {
        self.current
    }

    fn coeff_for(sample_rate: f32, time_ms: f32) -> f32 {
        let tau = (time_ms / 1000.0).max(1.0 / sample_rate.max(1.0));
        1.0 - (-1.0 / (sample_rate * tau)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_matches_repeated_next() {
        let mut a = SmoothedValue::new(0.0, 48_000.0, 20.0);
        let mut b = SmoothedValue::new(0.0, 48_000.0, 20.0);
        a.set_target(1.0);
        b.set_target(1.0);
        for _ in 0..128 {
            a.next();
        }
        b.advance(128);
        assert!((a.value() - b.value()).abs() < 1.0e-5);
    }
}
