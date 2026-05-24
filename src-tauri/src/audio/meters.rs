//! Peak + RMS meter with exponential decay.

#[derive(Debug, Clone, Copy)]
pub struct MeterValue {
    pub peak: f32,
    pub rms: f32,
}

impl MeterValue {
    pub const ZERO: Self = MeterValue { peak: 0.0, rms: 0.0 };

    pub fn peak_db(&self) -> f32 {
        amp_to_db(self.peak)
    }

    pub fn rms_db(&self) -> f32 {
        amp_to_db(self.rms)
    }
}

pub fn amp_to_db(amp: f32) -> f32 {
    if amp <= 1e-9 {
        -90.0
    } else {
        20.0 * amp.log10()
    }
}

pub struct Meter {
    peak: f32,
    rms_sum: f32,
    rms_count: u32,
    /// per-sample decay multiplier for peak.
    peak_decay: f32,
}

impl Meter {
    /// `sample_rate` is internal SR, `peak_release_ms` is time for peak to decay by ~99%.
    pub fn new(sample_rate: u32, peak_release_ms: f32) -> Self {
        let release_samples = (sample_rate as f32) * (peak_release_ms * 0.001).max(0.001);
        let peak_decay = (-1.0 / release_samples).exp();
        Self {
            peak: 0.0,
            rms_sum: 0.0,
            rms_count: 0,
            peak_decay,
        }
    }

    pub fn process(&mut self, samples: &[f32]) {
        for &s in samples {
            let abs = s.abs();
            if abs > self.peak {
                self.peak = abs;
            } else {
                self.peak *= self.peak_decay;
            }
            self.rms_sum += s * s;
            self.rms_count += 1;
        }
    }

    /// Drain accumulated samples into a MeterValue and reset the RMS window.
    pub fn snapshot(&mut self) -> MeterValue {
        let rms = if self.rms_count > 0 {
            (self.rms_sum / self.rms_count as f32).sqrt()
        } else {
            0.0
        };
        let value = MeterValue { peak: self.peak, rms };
        self.rms_sum = 0.0;
        self.rms_count = 0;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_signal_zero_meter() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.0; 480]);
        let v = m.snapshot();
        assert_eq!(v.peak, 0.0);
        assert_eq!(v.rms, 0.0);
    }

    #[test]
    fn unity_peak_detected() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[1.0]);
        let v = m.snapshot();
        assert!((v.peak - 1.0).abs() < 1e-6);
    }

    #[test]
    fn negative_peaks_use_absolute_value() {
        // Peak decays on non-max samples; use 1e-3 tolerance to allow for decay
        // over the 2 samples following the peak at -0.7.
        let mut m = Meter::new(48000, 100.0);
        m.process(&[-0.7, -0.2, 0.5]);
        let v = m.snapshot();
        assert!((v.peak - 0.7).abs() < 1e-3, "expected ~0.7, got {}", v.peak);
    }

    #[test]
    fn rms_of_constant_is_constant() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.5; 100]);
        let v = m.snapshot();
        assert!((v.rms - 0.5).abs() < 1e-6);
    }

    #[test]
    fn amp_to_db_anchor_values() {
        assert!((amp_to_db(1.0) - 0.0).abs() < 1e-6);
        assert!((amp_to_db(0.5) - (-6.020599)).abs() < 1e-3);
        assert_eq!(amp_to_db(0.0), -90.0);
    }

    #[test]
    fn snapshot_resets_rms() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.5; 100]);
        let _ = m.snapshot();
        m.process(&[0.0; 100]);
        let v = m.snapshot();
        assert!(v.rms < 1e-6);
    }
}
