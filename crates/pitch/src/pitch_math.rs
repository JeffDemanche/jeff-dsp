//! Pitch unit conversions and ratio helpers.

pub fn hz_to_midi(hz: f32, reference_a4: f32) -> f32 {
    if hz <= 0.0 || reference_a4 <= 0.0 {
        return 0.0;
    }
    69.0 + 12.0 * (hz / reference_a4).log2()
}

pub fn midi_to_hz(midi: f32, reference_a4: f32) -> f32 {
    reference_a4 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

pub fn clamp_ratio_to_correction_range(ratio: f32, range_semitones: f32) -> f32 {
    let range = range_semitones.max(0.0) + 0.15;
    let max_ratio = 2.0_f32.powf(range / 12.0).clamp(1.0, 2.0);
    let min_ratio = (1.0 / max_ratio).max(0.5);
    ratio.clamp(min_ratio, max_ratio)
}

pub fn one_pole_coeff(sample_rate: f32, time_ms: f32) -> f32 {
    let tau = (time_ms / 1000.0).max(1.0 / sample_rate.max(1.0));
    1.0 - (-1.0 / (sample_rate * tau)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_round_trip() {
        let midi = hz_to_midi(440.0, 440.0);
        assert!((midi - 69.0).abs() < 1.0e-4);
        let hz = midi_to_hz(69.0, 440.0);
        assert!((hz - 440.0).abs() < 1.0e-3);
    }

    #[test]
    fn ratio_clamp_symmetric() {
        let r = clamp_ratio_to_correction_range(4.0, 2.0);
        assert!(r <= 2.0);
        let r2 = clamp_ratio_to_correction_range(0.1, 2.0);
        assert!(r2 >= 0.5);
    }
}
