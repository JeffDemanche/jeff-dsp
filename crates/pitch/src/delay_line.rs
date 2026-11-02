//! Fractional delay line.

/// Circular buffer with fractional read (linear interpolation).
pub struct DelayLine {
    buffer: Vec<f32>,
    write: usize,
}

impl DelayLine {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity.max(2)],
            write: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.write = 0;
    }

    pub fn write_sample(&mut self, sample: f32) {
        self.buffer[self.write] = sample;
        self.write += 1;
        if self.write >= self.buffer.len() {
            self.write = 0;
        }
    }

    pub fn read_at(&self, delay_samples: f32) -> f32 {
        let len = self.buffer.len() as f32;
        let delay = delay_samples.clamp(1.0, len - 1.0);
        let read_pos = self.write as f32 - delay;
        let wrapped = if read_pos >= 0.0 {
            read_pos
        } else {
            read_pos + len * ((-read_pos / len).floor() + 1.0)
        };
        let index = wrapped % len;
        let i0 = index.floor() as usize % self.buffer.len();
        let i1 = (i0 + 1) % self.buffer.len();
        let frac = index - index.floor();
        self.buffer[i0] * (1.0 - frac) + self.buffer[i1] * frac
    }
}
