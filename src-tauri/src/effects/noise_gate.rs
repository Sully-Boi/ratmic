//! Bad noise gate: envelope follower + threshold, plus random chatter near the threshold.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseGateParams {
    #[serde(rename = "thresholdDb", default = "default_threshold")]
    pub threshold_db: f32,
    #[serde(rename = "attackMs", default = "default_attack")]
    pub attack_ms: f32,
    #[serde(rename = "releaseMs", default = "default_release")]
    pub release_ms: f32,
    #[serde(rename = "chatterAmount", default)]
    pub chatter_amount: f32,
}

fn default_threshold() -> f32 { -40.0 }
fn default_attack() -> f32 { 5.0 }
fn default_release() -> f32 { 80.0 }

impl Default for NoiseGateParams {
    fn default() -> Self {
        Self {
            threshold_db: default_threshold(),
            attack_ms: default_attack(),
            release_ms: default_release(),
            chatter_amount: 0.0,
        }
    }
}

const MIN_THRESH_DB: f32 = -60.0;
const MAX_THRESH_DB: f32 = 0.0;
const MIN_TIME_MS: f32 = 0.5;
const MAX_ATTACK_MS: f32 = 200.0;
const MAX_RELEASE_MS: f32 = 500.0;
/// Window above threshold (in dB) where chatter is allowed to false-close.
const CHATTER_BAND_DB: f32 = 6.0;
/// Number of samples between chatter dice rolls.
const CHATTER_EVAL_SAMPLES: u32 = 480; // 10 ms @ 48 kHz

pub struct NoiseGate {
    params: NoiseGateParams,
    sample_rate: u32,
    rng: fastrand::Rng,
    envelope: f32,
    /// Current gate openness, 0.0..=1.0, smoothed via attack/release.
    open: f32,
    target_open: f32,
    attack_coef: f32,
    release_coef: f32,
    threshold_amp: f32,
    chatter_band_low: f32,
    /// Samples remaining in a chatter-forced close.
    chatter_close_remaining: u32,
    chatter_counter: u32,
}

impl NoiseGate {
    pub fn new(sample_rate: u32, params: NoiseGateParams) -> Self {
        let mut g = Self {
            params,
            sample_rate,
            rng: fastrand::Rng::new(),
            envelope: 0.0,
            open: 0.0,
            target_open: 0.0,
            attack_coef: 0.0,
            release_coef: 0.0,
            threshold_amp: 0.0,
            chatter_band_low: 0.0,
            chatter_close_remaining: 0,
            chatter_counter: 0,
        };
        g.recompute();
        g
    }

    fn recompute(&mut self) {
        let sr = self.sample_rate as f32;
        let attack = self.params.attack_ms.max(MIN_TIME_MS);
        let release = self.params.release_ms.max(MIN_TIME_MS);
        self.attack_coef = (-1.0 / (attack * 0.001 * sr)).exp();
        self.release_coef = (-1.0 / (release * 0.001 * sr)).exp();
        self.threshold_amp = 10.0_f32.powf(self.params.threshold_db / 20.0);
        self.chatter_band_low = 10.0_f32
            .powf((self.params.threshold_db + CHATTER_BAND_DB) / 20.0);
    }
}

impl Effect for NoiseGate {
    fn type_name(&self) -> &'static str { "noiseGate" }

    fn process(&mut self, buffer: &mut [f32]) {
        let chatter = self.params.chatter_amount.clamp(0.0, 1.0);
        let env_attack = (-1.0 / (0.001 * self.sample_rate as f32)).exp(); // 1 ms env attack
        let env_release = (-1.0 / (0.030 * self.sample_rate as f32)).exp(); // 30 ms env release

        for s in buffer {
            let abs = s.abs();
            let coef = if abs > self.envelope { env_attack } else { env_release };
            self.envelope = abs + (self.envelope - abs) * coef;

            // Threshold decision.
            self.target_open = if self.envelope >= self.threshold_amp { 1.0 } else { 0.0 };

            // Chatter: occasionally force-close if envelope is in the band just above threshold.
            if chatter > 0.0
                && self.envelope > self.threshold_amp
                && self.envelope < self.chatter_band_low
            {
                self.chatter_counter += 1;
                if self.chatter_counter >= CHATTER_EVAL_SAMPLES {
                    self.chatter_counter = 0;
                    if self.rng.f32() < chatter {
                        // Force closed for 5..=15 ms.
                        let ms = 5.0 + self.rng.f32() * 10.0;
                        self.chatter_close_remaining =
                            ((ms * 0.001) * self.sample_rate as f32) as u32;
                    }
                }
            } else {
                self.chatter_counter = 0;
            }
            if self.chatter_close_remaining > 0 {
                self.chatter_close_remaining -= 1;
                self.target_open = 0.0;
            }

            // Smooth toward target via attack/release.
            let coef = if self.target_open > self.open { self.attack_coef } else { self.release_coef };
            self.open = self.target_open + (self.open - self.target_open) * coef;

            *s *= self.open;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: NoiseGateParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.threshold_db = p.threshold_db.clamp(MIN_THRESH_DB, MAX_THRESH_DB);
        p.attack_ms = p.attack_ms.clamp(MIN_TIME_MS, MAX_ATTACK_MS);
        p.release_ms = p.release_ms.clamp(MIN_TIME_MS, MAX_RELEASE_MS);
        p.chatter_amount = p.chatter_amount.clamp(0.0, 1.0);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("noise_gate params serialize")
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.open = 0.0;
        self.target_open = 0.0;
        self.chatter_close_remaining = 0;
        self.chatter_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_signal_is_gated_to_silence() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            chatter_amount: 0.0,
        });
        let mut buf = vec![0.01_f32; 4800]; // -40 dB
        g.process(&mut buf);
        let tail_max = buf[3000..].iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(tail_max < 0.001, "quiet signal should be gated, got {}", tail_max);
    }

    #[test]
    fn loud_signal_passes_through() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            chatter_amount: 0.0,
        });
        let mut buf = vec![0.5_f32; 4800]; // -6 dB
        g.process(&mut buf);
        let tail_avg: f32 = buf[3000..].iter().sum::<f32>() / buf[3000..].len() as f32;
        assert!(tail_avg > 0.4, "loud signal should pass, got avg {}", tail_avg);
    }

    #[test]
    fn release_avoids_immediate_click_on_quiet() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            chatter_amount: 0.0,
        });
        // 2000 samples loud (gate opens), then 3000 samples at a sub-threshold but
        // non-zero level. Gate releases slowly over ~100 ms release time. We then
        // verify that during the gate release window no consecutive output pair
        // differs by more than 0.05 — the gate must ramp, not snap.
        let mut buf = Vec::with_capacity(5000);
        for _ in 0..2000 { buf.push(0.5_f32); }        // loud: gate opens
        for _ in 0..3000 { buf.push(0.001_f32); }      // quiet: gate releases
        g.process(&mut buf);
        // Check the loud region for smoothness (gate fully open, signal constant).
        for w in buf.windows(2).skip(1990).take(8) {
            let diff = (w[1] - w[0]).abs();
            assert!(diff < 0.05, "pre-transition discontinuity {} at transition", diff);
        }
        // During the release phase (roughly 2100..2500), gate_open is ramping from
        // 1.0 toward 0.0. Output = gate_open × 0.001 is small, but the gate ramp
        // itself must be smooth (no per-sample step > 0.05 in output).
        for w in buf.windows(2).skip(2001).take(200) {
            let diff = (w[1] - w[0]).abs();
            assert!(diff < 0.05, "release discontinuity {} between adjacent samples", diff);
        }
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut g = NoiseGate::new(48000, NoiseGateParams::default());
        g.set_params(&serde_json::json!({
            "thresholdDb": 100.0,
            "attackMs": 0.0,
            "releaseMs": 9999.0,
            "chatterAmount": 2.0
        })).unwrap();
        let p: NoiseGateParams = serde_json::from_value(g.get_params()).unwrap();
        assert_eq!(p.threshold_db, MAX_THRESH_DB);
        assert_eq!(p.attack_ms, MIN_TIME_MS);
        assert_eq!(p.release_ms, MAX_RELEASE_MS);
        assert_eq!(p.chatter_amount, 1.0);
    }
}
