//! Streaming linear-interpolation resampler (mono f32).
//!
//! Converts a stream from `in_rate` to `out_rate`. Handles arbitrary input
//! chunk sizes across successive `process` calls and never allocates on the hot
//! path (the caller supplies the output Vec, which reaches steady capacity).
//!
//! Quality is intentionally modest (linear interpolation) — correct pitch is
//! what matters here, and the artifacts are inaudible for voice.

pub struct LinearResampler {
    /// Input samples advanced per output sample = in_rate / out_rate.
    step: f64,
    /// Fractional position measured from `prev` (0.0) toward the next input
    /// sample (1.0).
    pos: f64,
    /// Left endpoint of the current interpolation segment (previous input sample).
    prev: f32,
    /// True when in_rate == out_rate (pure passthrough).
    identity: bool,
}

impl LinearResampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        Self {
            step: in_rate as f64 / out_rate as f64,
            pos: 0.0,
            prev: 0.0,
            identity: in_rate == out_rate,
        }
    }

    pub fn is_identity(&self) -> bool {
        self.identity
    }

    /// Resample `input` into `out` (cleared first).
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        out.clear();
        if self.identity {
            out.extend_from_slice(input);
            return;
        }
        for &cur in input {
            // Emit every output sample whose position falls in [prev, cur).
            while self.pos < 1.0 {
                let frac = self.pos as f32;
                out.push(self.prev + (cur - self.prev) * frac);
                self.pos += self.step;
            }
            self.pos -= 1.0;
            self.prev = cur;
        }
    }

    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.prev = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_exact_passthrough() {
        let mut r = LinearResampler::new(48000, 48000);
        assert!(r.is_identity());
        let mut out = Vec::new();
        r.process(&[0.1, 0.2, -0.5, 0.7], &mut out);
        assert_eq!(out, vec![0.1, 0.2, -0.5, 0.7]);
    }

    #[test]
    fn upsample_roughly_doubles_length() {
        // 24k -> 48k: step = 0.5, expect ~2x output.
        let mut r = LinearResampler::new(24000, 48000);
        let input = vec![0.5_f32; 1000];
        let mut out = Vec::new();
        r.process(&input, &mut out);
        assert!(
            (out.len() as i64 - 2000).abs() <= 2,
            "expected ~2000 samples, got {}",
            out.len()
        );
    }

    #[test]
    fn downsample_roughly_halves_length() {
        // 48k -> 24k: step = 2.0, expect ~0.5x output.
        let mut r = LinearResampler::new(48000, 24000);
        let input = vec![0.5_f32; 1000];
        let mut out = Vec::new();
        r.process(&input, &mut out);
        assert!(
            (out.len() as i64 - 500).abs() <= 2,
            "expected ~500 samples, got {}",
            out.len()
        );
    }

    #[test]
    fn constant_signal_stays_constant_after_startup() {
        let mut r = LinearResampler::new(44100, 48000);
        let input = vec![0.8_f32; 2000];
        let mut out = Vec::new();
        r.process(&input, &mut out);
        // After the brief startup ramp, the tail must equal the constant input.
        for &s in out.iter().skip(10) {
            assert!((s - 0.8).abs() < 1e-4, "tail drifted: {}", s);
        }
    }

    #[test]
    fn streaming_across_calls_matches_single_call_length() {
        // Feeding two halves should produce ~the same total as one call.
        let mut r1 = LinearResampler::new(44100, 48000);
        let mut r2 = LinearResampler::new(44100, 48000);
        let input = vec![0.3_f32; 4410];
        let mut single = Vec::new();
        r1.process(&input, &mut single);
        let mut a = Vec::new();
        let mut b = Vec::new();
        r2.process(&input[..2205], &mut a);
        r2.process(&input[2205..], &mut b);
        let streamed = a.len() + b.len();
        assert!(
            (streamed as i64 - single.len() as i64).abs() <= 2,
            "streamed {} vs single {}",
            streamed,
            single.len()
        );
    }

    #[test]
    fn ramp_is_monotonic_when_upsampling() {
        // A rising ramp upsampled should stay non-decreasing.
        let mut r = LinearResampler::new(24000, 48000);
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let mut out = Vec::new();
        r.process(&input, &mut out);
        for w in out.windows(2).skip(5) {
            assert!(w[1] + 1e-6 >= w[0], "not monotonic: {} -> {}", w[0], w[1]);
        }
    }
}
