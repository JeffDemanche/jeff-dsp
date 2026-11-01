//! Prism DSP — C ABI for the JUCE plugin shell.

use shared_dsp::clamp;
use std::slice;

pub struct PrismDsp {
    sample_rate: f64,
    max_block_size: u32,
    gain: f32,
}

impl PrismDsp {
    fn new() -> Self {
        Self {
            sample_rate: 44_100.0,
            max_block_size: 0,
            gain: 1.0,
        }
    }

    fn prepare(&mut self, sample_rate: f64, max_block_size: u32) {
        self.sample_rate = sample_rate.max(1.0);
        self.max_block_size = max_block_size;
    }

    fn reset(&mut self) {
        // Stateless gain for now; hook for future filter/delay state.
    }

    fn set_gain(&mut self, gain_linear: f32) {
        self.gain = clamp(gain_linear, 0.0, 4.0);
    }

    fn process(&self, channels: &mut [&mut [f32]]) {
        let gain = self.gain;
        for channel in channels.iter_mut() {
            for sample in channel.iter_mut() {
                *sample *= gain;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn prism_dsp_create() -> *mut PrismDsp {
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
pub unsafe extern "C" fn prism_dsp_set_gain(dsp: *mut PrismDsp, gain_linear: f32) {
    if let Some(dsp) = dsp.as_mut() {
        dsp.set_gain(gain_linear);
    }
}

#[no_mangle]
pub unsafe extern "C" fn prism_dsp_process(
    dsp: *mut PrismDsp,
    channels: *const *mut f32,
    num_channels: u32,
    num_samples: u32,
) {
    let Some(dsp) = dsp.as_ref() else {
        return;
    };
    if channels.is_null() || num_channels == 0 || num_samples == 0 {
        return;
    }

    let channel_ptrs = slice::from_raw_parts(channels, num_channels as usize);
    let mut channel_slices: Vec<&mut [f32]> = channel_ptrs
        .iter()
        .map(|&ptr| slice::from_raw_parts_mut(ptr, num_samples as usize))
        .collect();

    dsp.process(&mut channel_slices);
}
