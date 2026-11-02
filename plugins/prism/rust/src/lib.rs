//! Prism DSP FFI — thin shell over `pitch::Prismizer`.

use pitch::{Prismizer, PrismizerMetering, PrismizerParams, MAX_VOICES};
use std::slice;

pub struct PrismDsp {
    sample_rate: f64,
    prismizer: Option<Prismizer>,
    /// Preallocated right-channel scratch for mono hosts.
    mono_right: Vec<f32>,
}

impl PrismDsp {
    fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            prismizer: None,
            mono_right: Vec::new(),
        }
    }

    fn ensure_prismizer(&mut self) {
        if self.prismizer.is_none() {
            let params = PrismizerParams::default();
            self.prismizer = Some(Prismizer::new(self.sample_rate as f32, &params));
        }
    }

    fn prepare(&mut self, sample_rate: f64, max_block_size: u32) {
        self.sample_rate = sample_rate.max(1.0);
        self.mono_right.resize(max_block_size as usize, 0.0);
        let params = PrismizerParams::default();
        self.prismizer = Some(Prismizer::new(self.sample_rate as f32, &params));
    }

    fn reset(&mut self) {
        if let Some(p) = self.prismizer.as_mut() {
            p.reset();
        }
    }

    fn set_params(&mut self, p: &PrismizerParams) {
        self.ensure_prismizer();
        if let Some(engine) = self.prismizer.as_mut() {
            engine.set_params(p);
        }
    }

    fn note_on(&mut self, midi_note: u8, velocity: f32) {
        self.ensure_prismizer();
        if let Some(engine) = self.prismizer.as_mut() {
            engine.note_on(midi_note, velocity);
        }
    }

    fn note_off(&mut self, midi_note: u8) {
        self.ensure_prismizer();
        if let Some(engine) = self.prismizer.as_mut() {
            engine.note_off(midi_note);
        }
    }

    fn all_sound_off(&mut self) {
        self.ensure_prismizer();
        if let Some(engine) = self.prismizer.as_mut() {
            engine.all_sound_off();
        }
    }

    fn metering(&self) -> PrismizerMetering {
        self.prismizer
            .as_ref()
            .map(|p| p.metering())
            .unwrap_or_default()
    }

    fn reported_latency_samples(&self) -> u32 {
        self.prismizer
            .as_ref()
            .map(|p| p.reported_latency_samples())
            .unwrap_or(0)
    }

    fn process(&mut self, left: &mut [f32], right: Option<&mut [f32]>) {
        let Some(engine) = self.prismizer.as_mut() else {
            return;
        };

        match right {
            Some(r) => {
                let n = left.len().min(r.len());
                engine.process_stereo(&mut left[..n], &mut r[..n]);
            }
            None => {
                let n = left.len();
                if self.mono_right.len() < n {
                    self.mono_right.resize(n, 0.0);
                }
                self.mono_right[..n].copy_from_slice(left);
                engine.process_stereo(left, &mut self.mono_right[..n]);
            }
        }
    }
}

fn params_from_c(p: &PrismDspParams) -> PrismizerParams {
    PrismizerParams {
        key_semitone: p.key_semitone.min(11),
        scale: p.scale.min(2),
        reference_hz: p.reference_hz,
        retune_speed_ms: p.retune_speed_ms,
        correction_amount: p.correction_amount,
        correction_range_semitones: p.correction_range_semitones,
        vibrato_preserve: p.vibrato_preserve,
        formant_follow: p.formant_follow,
        quantize_dead_zone_cents: p.quantize_dead_zone_cents,
        mix: p.mix,
        min_f0_hz: p.min_f0_hz,
        dry_level: p.dry_level,
        dry_correct: p.dry_correct != 0,
        attack_ms: p.attack_ms,
        decay_ms: p.decay_ms,
        sustain: p.sustain,
        release_ms: p.release_ms,
    }
}

#[repr(C)]
pub struct PrismDspParams {
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
    pub dry_level: f32,
    pub dry_correct: u8,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

#[repr(C)]
pub struct PrismDspMetering {
    pub detected_hz: f32,
    pub dry_target_hz: f32,
    pub num_voices: u32,
    pub voice_midi: [f32; MAX_VOICES],
    pub voice_target_hz: [f32; MAX_VOICES],
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_create() -> *mut PrismDsp {
    Box::into_raw(Box::new(PrismDsp::new()))
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_destroy(dsp: *mut PrismDsp) {
    if !dsp.is_null() {
        drop(Box::from_raw(dsp));
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_prepare(
    dsp: *mut PrismDsp,
    sample_rate: f64,
    max_block_size: u32,
) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.prepare(sample_rate, max_block_size);
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_reset(dsp: *mut PrismDsp) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_set_params(dsp: *mut PrismDsp, params: *const PrismDspParams) {
    let Some(dsp) = dsp.as_mut() else {
        return;
    };
    let Some(params) = params.as_ref() else {
        return;
    };
    dsp.set_params(&params_from_c(params));
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_note_on(dsp: *mut PrismDsp, midi_note: u8, velocity: f32) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.note_on(midi_note, velocity);
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_note_off(dsp: *mut PrismDsp, midi_note: u8) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.note_off(midi_note);
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_all_sound_off(dsp: *mut PrismDsp) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.all_sound_off();
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_get_metering(dsp: *const PrismDsp, out: *mut PrismDspMetering) {
    let Some(dsp) = dsp.as_ref() else {
        return;
    };
    let Some(out) = out.as_mut() else {
        return;
    };
    let m = dsp.metering();
    out.detected_hz = m.detected_hz;
    out.dry_target_hz = m.dry_target_hz;
    out.num_voices = m.num_voices;
    out.voice_midi = m.voice_midi;
    out.voice_target_hz = m.voice_target_hz;
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_reported_latency_samples(dsp: *const PrismDsp) -> u32 {
    dsp.as_ref()
        .map(|d| d.reported_latency_samples())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_process(
    dsp: *mut PrismDsp,
    channels: *const *mut f32,
    num_channels: u32,
    num_samples: u32,
) {
    let Some(dsp) = dsp.as_mut() else {
        return;
    };
    if channels.is_null() || num_channels == 0 || num_samples == 0 {
        return;
    }

    let channel_ptrs = slice::from_raw_parts(channels, num_channels as usize);
    if channel_ptrs[0].is_null() {
        return;
    }

    let left = slice::from_raw_parts_mut(channel_ptrs[0], num_samples as usize);
    if num_channels >= 2 && !channel_ptrs[1].is_null() {
        let right = slice::from_raw_parts_mut(channel_ptrs[1], num_samples as usize);
        dsp.process(left, Some(right));
    } else {
        dsp.process(left, None);
    }
}
