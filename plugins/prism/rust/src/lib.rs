//! Prism DSP FFI — thin shell over `pitch::Corrector<ScalePitchTarget>`.

use pitch::{Corrector, CorrectorParams, ScalePitchTarget};
use std::slice;

pub struct PrismDsp {
    sample_rate: f64,
    corrector: Option<Corrector<ScalePitchTarget>>,
    /// Preallocated right-channel scratch for mono hosts.
    mono_right: Vec<f32>,
}

impl PrismDsp {
    fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            corrector: None,
            mono_right: Vec::new(),
        }
    }

    fn ensure_corrector(&mut self) {
        if self.corrector.is_none() {
            let params = CorrectorParams::default();
            self.corrector = Some(Corrector::new_scale(self.sample_rate as f32, &params));
        }
    }

    fn prepare(&mut self, sample_rate: f64, max_block_size: u32) {
        self.sample_rate = sample_rate.max(1.0);
        self.mono_right.resize(max_block_size as usize, 0.0);
        let params = CorrectorParams::default();
        self.corrector = Some(Corrector::new_scale(self.sample_rate as f32, &params));
    }

    fn reset(&mut self) {
        if let Some(c) = self.corrector.as_mut() {
            c.reset();
        }
    }

    fn set_params(&mut self, p: &CorrectorParams) {
        self.ensure_corrector();
        if let Some(c) = self.corrector.as_mut() {
            c.set_key_scale(p.key_semitone, p.scale);
            c.set_params(p);
        }
    }

    fn reported_latency_samples(&self) -> u32 {
        self.corrector
            .as_ref()
            .map(|c| c.reported_latency_samples())
            .unwrap_or(0)
    }

    fn process(&mut self, left: &mut [f32], right: Option<&mut [f32]>) {
        let Some(corrector) = self.corrector.as_mut() else {
            return;
        };

        match right {
            Some(r) => {
                let n = left.len().min(r.len());
                corrector.process_stereo(&mut left[..n], &mut r[..n]);
            }
            None => {
                let n = left.len();
                if self.mono_right.len() < n {
                    self.mono_right.resize(n, 0.0);
                }
                self.mono_right[..n].copy_from_slice(left);
                corrector.process_stereo(left, &mut self.mono_right[..n]);
            }
        }
    }
}

fn params_from_c(p: &PrismDspParams) -> CorrectorParams {
    CorrectorParams {
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
