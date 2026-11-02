//! Scale degree snapping with hysteresis and dead zone.

/// Quantize hysteresis (cents).
pub const QUANTIZE_HYSTERESIS_CENTS: f32 = 25.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    Chromatic = 0,
    Major = 1,
    Minor = 2,
}

impl ScaleKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Major,
            2 => Self::Minor,
            _ => Self::Chromatic,
        }
    }

    pub fn intervals(self) -> &'static [u8] {
        match self {
            Self::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::Minor => &[0, 2, 3, 5, 7, 8, 10],
        }
    }
}

pub fn nearest_scale_midi(
    midi: f32,
    key_semitone: u8,
    scale: ScaleKind,
    range_semitones: f32,
) -> Option<f32> {
    let intervals = scale.intervals();
    let range = range_semitones.max(0.0);
    let mut best_midi = midi;
    let mut best_dist = f32::MAX;
    let base_octave = (midi / 12.0).floor();
    for octave_offset in -1i32..=1 {
        let octave_base = (base_octave + octave_offset as f32) * 12.0;
        for &interval in intervals {
            let candidate = octave_base + key_semitone as f32 + interval as f32;
            let dist = (candidate - midi).abs();
            if dist <= range && dist < best_dist {
                best_dist = dist;
                best_midi = candidate;
            }
        }
    }
    if best_dist > range {
        None
    } else {
        Some(best_midi)
    }
}

pub fn quantize_scale_degree_with_hysteresis(
    midi: f32,
    key_semitone: u8,
    scale: ScaleKind,
    range_semitones: f32,
    previous_quantized: f32,
) -> Option<f32> {
    let nearest = nearest_scale_midi(midi, key_semitone, scale, range_semitones)?;
    let range = range_semitones.max(0.0);
    let hyst = QUANTIZE_HYSTERESIS_CENTS / 100.0;
    if previous_quantized > 0.0 {
        let dist_prev = (midi - previous_quantized).abs();
        let dist_near = (midi - nearest).abs();
        if dist_prev <= range && dist_prev <= dist_near + hyst {
            return Some(previous_quantized);
        }
    }
    Some(nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch_math::hz_to_midi;

    #[test]
    fn quantize_snaps_within_range() {
        let midi = hz_to_midi(277.18, 440.0); // C#4
        let snapped = nearest_scale_midi(midi, 0, ScaleKind::Major, 2.0).unwrap();
        let c4 = 60.0_f32;
        let d4 = 62.0_f32;
        assert!(
            (snapped - c4).abs() < 0.01 || (snapped - d4).abs() < 0.01,
            "snapped {snapped}"
        );
    }
}
