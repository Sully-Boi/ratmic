//! RBJ-cookbook biquad filter.
//!
//! Direct form I, single-sample-per-call. State: x[n-1], x[n-2], y[n-1], y[n-2].
//!
//! Coefficients are pre-normalized by a0 so the runtime loop is fewer ops.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub enum FilterKind {
    LowPass,
    HighPass,
    PeakEq { gain_db: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoefs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefs {
    pub fn identity() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }

    pub fn design(kind: FilterKind, freq_hz: f32, q: f32, sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        // Clamp frequency to a safe range to avoid pathological coefficients.
        let f0 = freq_hz.clamp(10.0, sr * 0.45);
        let q = q.max(0.001);
        let w0 = 2.0 * PI * f0 / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            FilterKind::LowPass => {
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterKind::HighPass => {
                let b1 = -(1.0 + cos_w0);
                let b0 = (1.0 + cos_w0) / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterKind::PeakEq { gain_db } => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                (
                    1.0 + alpha * a,
                    -2.0 * cos_w0,
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * cos_w0,
                    1.0 - alpha / a,
                )
            }
        };

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    pub coefs: BiquadCoefs,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new(coefs: BiquadCoefs) -> Self {
        Self { coefs, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    pub fn identity() -> Self {
        Self::new(BiquadCoefs::identity())
    }

    pub fn set_coefs(&mut self, coefs: BiquadCoefs) {
        self.coefs = coefs;
        // Don't clear state — coefficient changes mid-stream should be smooth.
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    pub fn process_sample(&mut self, x: f32) -> f32 {
        let c = &self.coefs;
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2
            - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for s in buffer {
            *s = self.process_sample(*s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a biquad with a sine of given frequency and return peak amplitude
    /// of the steady-state output (after a warmup).
    ///
    /// Uses a continuous sample counter so the phase is never restarted between
    /// the warmup and measurement windows — a phase discontinuity at the boundary
    /// injects a transient that can dominate the peak reading for stopband tests.
    fn measure_response(b: &mut Biquad, freq_hz: f32, sample_rate: u32, samples: usize) -> f32 {
        let sr = sample_rate as f32;
        const WARMUP: usize = 4096;
        let mut peak = 0.0_f32;
        for n in 0..(WARMUP + samples) {
            let t = n as f32 / sr;
            let x = (2.0 * PI * freq_hz * t).sin();
            let y = b.process_sample(x);
            if n >= WARMUP {
                peak = peak.max(y.abs());
            }
        }
        peak
    }

    #[test]
    fn identity_passes_signal() {
        let mut b = Biquad::identity();
        let mut buf = vec![0.1, 0.2, -0.5, 0.7];
        b.process_buffer(&mut buf);
        // Identity should leave samples close to original (no perfect equality due to
        // the recursive form, but it should not move them by more than ~1e-6).
        let expected = [0.1, 0.2, -0.5, 0.7];
        for (a, e) in buf.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-6, "identity drifted: {} vs {}", a, e);
        }
    }

    #[test]
    fn lowpass_attenuates_high_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 500.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let high = measure_response(&mut b, 8000.0, 48000, 4096);
        // 8 kHz is well above 500 Hz cutoff — expect heavy attenuation.
        assert!(high < 0.1, "8 kHz peak should be < 0.1, got {}", high);
    }

    #[test]
    fn lowpass_passes_low_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 2000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let low = measure_response(&mut b, 200.0, 48000, 4096);
        // 200 Hz is well below 2 kHz cutoff — should pass near unity.
        assert!(low > 0.9, "200 Hz peak should be > 0.9, got {}", low);
    }

    #[test]
    fn highpass_attenuates_low_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::HighPass, 1000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let low = measure_response(&mut b, 100.0, 48000, 4096);
        assert!(low < 0.1, "100 Hz peak should be < 0.1, got {}", low);
    }

    #[test]
    fn highpass_passes_high_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::HighPass, 500.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let high = measure_response(&mut b, 6000.0, 48000, 4096);
        assert!(high > 0.9, "6 kHz peak should be > 0.9, got {}", high);
    }

    #[test]
    fn peak_eq_boosts_center_frequency() {
        let coefs = BiquadCoefs::design(
            FilterKind::PeakEq { gain_db: 12.0 },
            1000.0,
            1.0,
            48000,
        );
        let mut b = Biquad::new(coefs);
        let center = measure_response(&mut b, 1000.0, 48000, 4096);
        // +12 dB is ~3.98x amplitude.
        assert!(center > 3.0, "1 kHz peak with +12 dB should be > 3.0, got {}", center);
    }

    #[test]
    fn reset_clears_state() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 1000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        b.process_sample(1.0);
        b.process_sample(1.0);
        assert_ne!(b.y1, 0.0);
        b.reset();
        assert_eq!(b.y1, 0.0);
        assert_eq!(b.y2, 0.0);
        assert_eq!(b.x1, 0.0);
        assert_eq!(b.x2, 0.0);
    }
}
