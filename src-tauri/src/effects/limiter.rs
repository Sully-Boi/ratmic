//! Peak limiter with smooth attack/release.
//!
//! No lookahead: a single-sample peak detector with exponential envelope.
//! Sufficient for ear-safety; for "brick-wall" guarantees we'd need lookahead.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterParams {
    #[serde(rename = "ceilingDb", default = "default_ceiling")]
    pub ceiling_db: f32,
    #[serde(rename = "releaseMs", default = "default_release")]
    pub release_ms: f32,
}

fn default_ceiling() -> f32 { -3.0 }
fn default_release() -> f32 { 80.0 }

impl Default for LimiterParams {
    fn default() -> Self {
        Self { ceiling_db: default_ceiling(), release_ms: default_release() }
    }
}

const MIN_CEILING_DB: f32 = -24.0;
const MAX_CEILING_DB: f32 = 0.0;
const MIN_RELEASE_MS: f32 = 1.0;
const MAX_RELEASE_MS: f32 = 500.0;
const ATTACK_MS: f32 = 5.0;

pub struct Limiter {
    params: LimiterParams,
    sample_rate: u32,
    ceiling_amp: f32,
    gain: f32,
    attack_coef: f32,
    release_coef: f32,
    /// True if any sample in the most recent process() call required limiting.
    pub was_active: bool,
}

impl Limiter {
    pub fn new(sample_rate: u32, params: LimiterParams) -> Self {
        let mut l = Self {
            params,
            sample_rate,
            ceiling_amp: 1.0,
            gain: 1.0,
            attack_coef: 0.0,
            release_coef: 0.0,
            was_active: false,
        };
        l.recompute();
        l
    }

    fn recompute(&mut self) {
        let sr = self.sample_rate as f32;
        self.ceiling_amp = 10.0_f32.powf(self.params.ceiling_db / 20.0);
        self.attack_coef = (-1.0 / (ATTACK_MS * 0.001 * sr)).exp();
        self.release_coef = (-1.0 / (self.params.release_ms * 0.001 * sr)).exp();
    }
}

impl Effect for Limiter {
    fn type_name(&self) -> &'static str { "limiter" }

    fn process(&mut self, buffer: &mut [f32]) {
        let ceiling = self.ceiling_amp;
        let attack = self.attack_coef;
        let release = self.release_coef;
        let mut gain = self.gain;
        let mut active = false;
        for s in buffer {
            let abs = s.abs();
            let target_gain = if abs * gain > ceiling {
                active = true;
                ceiling / abs
            } else {
                1.0
            };
            let coef = if target_gain < gain { attack } else { release };
            gain = target_gain + (gain - target_gain) * coef;
            *s *= gain;
            if s.abs() > ceiling {
                *s = s.signum() * ceiling;
            }
        }
        self.gain = gain;
        self.was_active = active;
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: LimiterParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.ceiling_db = p.ceiling_db.clamp(MIN_CEILING_DB, MAX_CEILING_DB);
        p.release_ms = p.release_ms.clamp(MIN_RELEASE_MS, MAX_RELEASE_MS);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("limiter params serialize")
    }

    fn reset(&mut self) {
        self.gain = 1.0;
        self.was_active = false;
    }

    fn limiter_was_active(&self) -> Option<bool> {
        Some(self.was_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_signal_passes_through_unchanged() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        let mut buf = vec![0.1; 256];
        let original = buf.clone();
        l.process(&mut buf);
        for (a, b) in buf.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
        assert!(!l.was_active);
    }

    #[test]
    fn loud_signal_is_clamped_to_ceiling() {
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        let ceiling_amp = 10.0_f32.powf(-3.0 / 20.0);
        let mut buf = vec![0.95; 4800]; // 100 ms of loud signal
        l.process(&mut buf);
        // After attack settles, samples should be at or below ceiling.
        for (i, s) in buf.iter().enumerate().skip(1000) {
            assert!(
                s.abs() <= ceiling_amp + 1e-3,
                "sample {} = {} exceeds ceiling {}",
                i, s, ceiling_amp
            );
        }
        assert!(l.was_active);
    }

    #[test]
    fn negative_peaks_also_clamped() {
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        let ceiling_amp = 10.0_f32.powf(-3.0 / 20.0);
        let mut buf = vec![-0.95; 4800];
        l.process(&mut buf);
        for s in buf.iter().skip(1000) {
            assert!(s.abs() <= ceiling_amp + 1e-3);
        }
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        l.set_params(&serde_json::json!({ "ceilingDb": 10.0, "releaseMs": 99999.0 })).unwrap();
        let p: LimiterParams = serde_json::from_value(l.get_params()).unwrap();
        assert_eq!(p.ceiling_db, MAX_CEILING_DB);
        assert_eq!(p.release_ms, MAX_RELEASE_MS);
    }

    #[test]
    fn reset_clears_gain_state() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        let mut buf = vec![0.95; 1000];
        l.process(&mut buf);
        assert!(l.gain < 1.0);
        l.reset();
        assert_eq!(l.gain, 1.0);
        assert!(!l.was_active);
    }

    #[test]
    fn limiter_reports_activity_via_trait() {
        use super::super::Effect;
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        // Quiet signal — not active.
        let mut buf = vec![0.1; 256];
        l.process(&mut buf);
        let trait_obj: &dyn Effect = &l;
        assert_eq!(trait_obj.limiter_was_active(), Some(false));
        // Loud signal — active.
        let mut buf = vec![0.95; 256];
        l.process(&mut buf);
        let trait_obj: &dyn Effect = &l;
        assert_eq!(trait_obj.limiter_was_active(), Some(true));
    }
}
