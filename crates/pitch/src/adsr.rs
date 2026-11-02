//! Linear per-voice amplitude ADSR (exact stage times in ms).

/// Envelope timing / sustain level.
#[derive(Debug, Clone, Copy)]
pub struct AdsrParams {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack_ms: 5.0,
            decay_ms: 0.0,
            sustain: 1.0,
            release_ms: 80.0,
        }
    }
}

/// ADSR stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

const IDLE_FLOOR: f32 = 1e-4;

/// Realtime-safe linear ADSR. Deltas are set on stage entry from current params.
#[derive(Debug, Clone)]
pub struct Adsr {
    sample_rate: f32,
    stage: AdsrStage,
    level: f32,
    delta: f32,
    target: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            stage: AdsrStage::Idle,
            level: 0.0,
            delta: 0.0,
            target: 0.0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
    }

    pub fn stage(&self) -> AdsrStage {
        self.stage
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn is_active(&self) -> bool {
        self.stage != AdsrStage::Idle
    }

    pub fn is_releasing(&self) -> bool {
        self.stage == AdsrStage::Release
    }

    pub fn reset(&mut self) {
        self.stage = AdsrStage::Idle;
        self.level = 0.0;
        self.delta = 0.0;
        self.target = 0.0;
    }

    pub fn note_on(&mut self, params: &AdsrParams) {
        self.enter_attack(params);
    }

    pub fn note_off(&mut self, params: &AdsrParams) {
        if self.stage == AdsrStage::Idle {
            return;
        }
        self.enter_release(params);
    }

    /// Advance one sample; returns current amplitude gain in `[0, 1]`.
    pub fn process(&mut self, params: &AdsrParams) -> f32 {
        match self.stage {
            AdsrStage::Idle => 0.0,
            AdsrStage::Attack => {
                self.level = (self.level + self.delta).min(self.target);
                if self.level >= self.target - IDLE_FLOOR {
                    self.level = 1.0;
                    self.enter_decay(params);
                }
                self.level.clamp(0.0, 1.0)
            }
            AdsrStage::Decay => {
                // Approach sustain from above or below.
                let next = self.level + self.delta;
                let reached = if self.delta <= 0.0 {
                    next <= self.target + IDLE_FLOOR
                } else {
                    next >= self.target - IDLE_FLOOR
                };
                if reached {
                    self.level = params.sustain.clamp(0.0, 1.0);
                    self.stage = AdsrStage::Sustain;
                    self.delta = 0.0;
                } else {
                    self.level = next;
                }
                self.level.clamp(0.0, 1.0)
            }
            AdsrStage::Sustain => {
                self.level = params.sustain.clamp(0.0, 1.0);
                self.level
            }
            AdsrStage::Release => {
                self.level = (self.level + self.delta).max(0.0);
                if self.level <= IDLE_FLOOR {
                    self.reset();
                    return 0.0;
                }
                self.level.clamp(0.0, 1.0)
            }
        }
    }

    fn samples_for_ms(&self, ms: f32) -> f32 {
        (ms.max(0.0) * 0.001 * self.sample_rate).max(1.0)
    }

    fn enter_attack(&mut self, params: &AdsrParams) {
        let attack_ms = params.attack_ms.max(0.0);
        if attack_ms <= 0.0 {
            self.level = 1.0;
            self.enter_decay(params);
            return;
        }
        self.stage = AdsrStage::Attack;
        self.target = 1.0;
        self.delta = (1.0 - self.level) / self.samples_for_ms(attack_ms);
        if self.delta <= 0.0 {
            self.level = 1.0;
            self.enter_decay(params);
        }
    }

    fn enter_decay(&mut self, params: &AdsrParams) {
        let sustain = params.sustain.clamp(0.0, 1.0);
        let decay_ms = params.decay_ms.max(0.0);
        if decay_ms <= 0.0 {
            self.level = sustain;
            self.stage = AdsrStage::Sustain;
            self.delta = 0.0;
            self.target = sustain;
            return;
        }
        self.stage = AdsrStage::Decay;
        self.target = sustain;
        self.delta = (sustain - self.level) / self.samples_for_ms(decay_ms);
        if (sustain - self.level).abs() <= IDLE_FLOOR {
            self.level = sustain;
            self.stage = AdsrStage::Sustain;
            self.delta = 0.0;
        }
    }

    fn enter_release(&mut self, params: &AdsrParams) {
        let release_ms = params.release_ms.max(0.0);
        if release_ms <= 0.0 || self.level <= IDLE_FLOOR {
            self.reset();
            return;
        }
        self.stage = AdsrStage::Release;
        self.target = 0.0;
        self.delta = -self.level / self.samples_for_ms(release_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(a: f32, d: f32, s: f32, r: f32) -> AdsrParams {
        AdsrParams {
            attack_ms: a,
            decay_ms: d,
            sustain: s,
            release_ms: r,
        }
    }

    #[test]
    fn attack_decay_sustain_release_sequence() {
        let sr = 1_000.0_f32; // 1 sample ≈ 1 ms
        let mut env = Adsr::new(sr);
        let p = params(10.0, 10.0, 0.5, 10.0);

        env.note_on(&p);
        assert_eq!(env.stage(), AdsrStage::Attack);

        for _ in 0..10 {
            env.process(&p);
        }
        assert!(
            env.stage() == AdsrStage::Decay || env.stage() == AdsrStage::Sustain,
            "expected decay/sustain after attack, got {:?}",
            env.stage()
        );

        for _ in 0..15 {
            env.process(&p);
        }
        assert_eq!(env.stage(), AdsrStage::Sustain);
        assert!((env.level() - 0.5).abs() < 0.02);

        env.note_off(&p);
        assert_eq!(env.stage(), AdsrStage::Release);
        for _ in 0..20 {
            env.process(&p);
        }
        assert_eq!(env.stage(), AdsrStage::Idle);
        assert_eq!(env.level(), 0.0);
    }

    #[test]
    fn zero_time_stages_skip() {
        let mut env = Adsr::new(48_000.0);
        let p = params(0.0, 0.0, 0.75, 0.0);
        env.note_on(&p);
        let g = env.process(&p);
        assert_eq!(env.stage(), AdsrStage::Sustain);
        assert!((g - 0.75).abs() < 1e-5);

        env.note_off(&p);
        assert_eq!(env.stage(), AdsrStage::Idle);
    }

    #[test]
    fn note_off_during_attack_enters_release() {
        let mut env = Adsr::new(1_000.0);
        let p = params(100.0, 0.0, 1.0, 50.0);
        env.note_on(&p);
        for _ in 0..10 {
            env.process(&p);
        }
        assert_eq!(env.stage(), AdsrStage::Attack);
        let level_before = env.level();
        assert!(level_before > 0.0 && level_before < 1.0);

        env.note_off(&p);
        assert_eq!(env.stage(), AdsrStage::Release);
        let g = env.process(&p);
        assert!(g < level_before);
        assert!(env.is_active());
    }

    #[test]
    fn idle_after_release_floor() {
        let mut env = Adsr::new(1_000.0);
        let p = params(0.0, 0.0, 1.0, 5.0);
        env.note_on(&p);
        env.process(&p);
        env.note_off(&p);
        for _ in 0..10 {
            env.process(&p);
        }
        assert!(!env.is_active());
        assert_eq!(env.level(), 0.0);
    }
}
