//! Bandpass + optional mid-boost: HP → LP → peak EQ.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::biquad::{Biquad, BiquadCoefs, FilterKind};
use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandpassParams {
    #[serde(rename = "lowCutHz", default = "default_low_cut")]
    pub low_cut_hz: f32,
    #[serde(rename = "highCutHz", default = "default_high_cut")]
    pub high_cut_hz: f32,
    #[serde(rename = "midBoostDb", default)]
    pub mid_boost_db: f32,
}

fn default_low_cut() -> f32 { 100.0 }
fn default_high_cut() -> f32 { 8000.0 }

impl Default for BandpassParams {
    fn default() -> Self {
        Self {
            low_cut_hz: default_low_cut(),
            high_cut_hz: default_high_cut(),
            mid_boost_db: 0.0,
        }
    }
}

const MIN_LOW: f32 = 20.0;
const MAX_LOW: f32 = 8000.0;
const MIN_HIGH: f32 = 200.0;
const MAX_HIGH: f32 = 20_000.0;
const MIN_BOOST_DB: f32 = -12.0;
const MAX_BOOST_DB: f32 = 12.0;

pub struct Bandpass {
    params: BandpassParams,
    sample_rate: u32,
    hp: Biquad,
    lp: Biquad,
    peak: Biquad,
}

impl Bandpass {
    pub fn new(sample_rate: u32, params: BandpassParams) -> Self {
        let mut b = Self {
            params,
            sample_rate,
            hp: Biquad::identity(),
            lp: Biquad::identity(),
            peak: Biquad::identity(),
        };
        b.recompute();
        b
    }

    fn recompute(&mut self) {
        let hp = BiquadCoefs::design(FilterKind::HighPass, self.params.low_cut_hz, 0.707, self.sample_rate);
        let lp = BiquadCoefs::design(FilterKind::LowPass, self.params.high_cut_hz, 0.707, self.sample_rate);
        self.hp.set_coefs(hp);
        self.lp.set_coefs(lp);
        if self.params.mid_boost_db.abs() < 0.01 {
            self.peak.set_coefs(BiquadCoefs::identity());
        } else {
            let mid = (self.params.low_cut_hz * self.params.high_cut_hz).sqrt();
            let peak = BiquadCoefs::design(
                FilterKind::PeakEq { gain_db: self.params.mid_boost_db },
                mid, 1.0, self.sample_rate,
            );
            self.peak.set_coefs(peak);
        }
    }
}

impl Effect for Bandpass {
    fn type_name(&self) -> &'static str { "bandpass" }

    fn process(&mut self, buffer: &mut [f32]) {
        for s in buffer {
            let mut y = self.hp.process_sample(*s);
            y = self.lp.process_sample(y);
            y = self.peak.process_sample(y);
            *s = y;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: BandpassParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.low_cut_hz = p.low_cut_hz.clamp(MIN_LOW, MAX_LOW);
        p.high_cut_hz = p.high_cut_hz.clamp(MIN_HIGH, MAX_HIGH);
        // Ensure low < high.
        if p.low_cut_hz >= p.high_cut_hz {
            p.high_cut_hz = (p.low_cut_hz + 100.0).min(MAX_HIGH);
        }
        p.mid_boost_db = p.mid_boost_db.clamp(MIN_BOOST_DB, MAX_BOOST_DB);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("bandpass params serialize")
    }

    fn reset(&mut self) {
        self.hp.reset();
        self.lp.reset();
        self.peak.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn measure_peak(b: &mut Bandpass, freq_hz: f32) -> f32 {
        // Continuous phase across warmup + measurement to avoid a discontinuity
        // at the boundary (which manifests as a fake "peak" dominating the read).
        const WARMUP: usize = 4096;
        const MEASURE: usize = 4096;
        let mut peak = 0.0_f32;
        for n in 0..(WARMUP + MEASURE) {
            let t = n as f32 / 48000.0;
            let x = (2.0 * PI * freq_hz * t).sin();
            let mut buf = [x];
            b.process(&mut buf);
            if n >= WARMUP {
                peak = peak.max(buf[0].abs());
            }
        }
        peak
    }

    #[test]
    fn telephone_band_passes_mid_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let mid = measure_peak(&mut b, 1000.0);
        assert!(mid > 0.7, "1 kHz should pass, got {}", mid);
    }

    #[test]
    fn telephone_band_attenuates_low_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let low = measure_peak(&mut b, 50.0);
        assert!(low < 0.2, "50 Hz should be attenuated, got {}", low);
    }

    #[test]
    fn telephone_band_attenuates_high_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let high = measure_peak(&mut b, 12000.0);
        assert!(high < 0.2, "12 kHz should be attenuated, got {}", high);
    }

    #[test]
    fn mid_boost_amplifies_band_center() {
        let mut flat = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let mut boosted = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 9.0,
        });
        // Geometric mean of 300 & 3400 is ~1010 Hz.
        let flat_peak = measure_peak(&mut flat, 1010.0);
        let boosted_peak = measure_peak(&mut boosted, 1010.0);
        assert!(
            boosted_peak > flat_peak * 1.5,
            "+9 dB boost should be > 1.5x flat (got {} vs {})",
            boosted_peak, flat_peak
        );
    }

    #[test]
    fn params_clamp_and_enforce_low_lt_high() {
        let mut b = Bandpass::new(48000, BandpassParams::default());
        b.set_params(&serde_json::json!({
            "lowCutHz": 5000.0,
            "highCutHz": 2000.0,
            "midBoostDb": 50.0
        })).unwrap();
        let p: BandpassParams = serde_json::from_value(b.get_params()).unwrap();
        assert!(p.low_cut_hz < p.high_cut_hz);
        assert_eq!(p.mid_boost_db, MAX_BOOST_DB);
    }
}
