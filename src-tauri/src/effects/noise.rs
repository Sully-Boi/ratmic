//! White noise + hum + crackle, optionally gated by input level.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::f32::consts::PI;

use super::Effect;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NoiseGateMode {
    Always,
    OnSpeech,
}

impl Default for NoiseGateMode {
    fn default() -> Self { Self::Always }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseParams {
    #[serde(rename = "whiteAmount", default)]
    pub white_amount: f32,
    #[serde(rename = "humAmount", default)]
    pub hum_amount: f32,
    #[serde(rename = "humHz", default = "default_hum_hz")]
    pub hum_hz: f32,
    #[serde(rename = "crackleRate", default)]
    pub crackle_rate: f32, // events per second
    #[serde(rename = "gateMode", default)]
    pub gate_mode: NoiseGateMode,
    #[serde(rename = "speechThresholdDb", default = "default_speech_thresh")]
    pub speech_threshold_db: f32,
}

fn default_hum_hz() -> f32 { 60.0 }
fn default_speech_thresh() -> f32 { -40.0 }

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            white_amount: 0.0,
            hum_amount: 0.0,
            hum_hz: default_hum_hz(),
            crackle_rate: 0.0,
            gate_mode: NoiseGateMode::Always,
            speech_threshold_db: default_speech_thresh(),
        }
    }
}

const MAX_WHITE: f32 = 0.5;
const MAX_HUM: f32 = 0.5;
const MIN_HUM_HZ: f32 = 40.0;
const MAX_HUM_HZ: f32 = 120.0;
const MAX_CRACKLE_RATE: f32 = 50.0;

pub struct Noise {
    params: NoiseParams,
    sample_rate: u32,
    rng: fastrand::Rng,
    /// Hum phase accumulator, radians.
    hum_phase: f32,
    /// Smoothed input envelope for gate-mode decisions.
    envelope: f32,
    env_attack: f32,
    env_release: f32,
}

impl Noise {
    pub fn new(sample_rate: u32, params: NoiseParams) -> Self {
        let sr = sample_rate as f32;
        // ~5 ms attack, ~80 ms release on the envelope follower.
        let env_attack = (-1.0 / (0.005 * sr)).exp();
        let env_release = (-1.0 / (0.080 * sr)).exp();
        Self {
            params,
            sample_rate,
            rng: fastrand::Rng::new(),
            hum_phase: 0.0,
            envelope: 0.0,
            env_attack,
            env_release,
        }
    }

    fn next_white(&mut self) -> f32 {
        // Uniform in [-1.0, 1.0)
        self.rng.f32() * 2.0 - 1.0
    }

    fn maybe_crackle(&mut self) -> f32 {
        let rate = self.params.crackle_rate;
        if rate <= 0.0 { return 0.0; }
        let p = rate / self.sample_rate as f32;
        if self.rng.f32() < p {
            // Short impulse, +/- random amplitude in [0.3, 0.8].
            let amp = 0.3 + self.rng.f32() * 0.5;
            if self.rng.bool() { amp } else { -amp }
        } else {
            0.0
        }
    }
}

impl Effect for Noise {
    fn type_name(&self) -> &'static str { "noise" }

    fn process(&mut self, buffer: &mut [f32]) {
        let white = self.params.white_amount.clamp(0.0, MAX_WHITE);
        let hum = self.params.hum_amount.clamp(0.0, MAX_HUM);
        let hum_hz = self.params.hum_hz.clamp(MIN_HUM_HZ, MAX_HUM_HZ);
        let phase_inc = 2.0 * PI * hum_hz / self.sample_rate as f32;
        let speech_thresh_amp = 10.0_f32.powf(self.params.speech_threshold_db / 20.0);
        let gate_on_speech = matches!(self.params.gate_mode, NoiseGateMode::OnSpeech);
        let env_attack = self.env_attack;
        let env_release = self.env_release;

        for s in buffer {
            let abs = s.abs();
            // Envelope follower (5 ms attack / 80 ms release, set in new()).
            let coef = if abs > self.envelope { env_attack } else { env_release };
            self.envelope = abs + (self.envelope - abs) * coef;

            // Gate decision.
            let gate_open = !gate_on_speech || self.envelope > speech_thresh_amp;

            let mut noise = 0.0;
            if gate_open {
                if white > 0.0 {
                    noise += self.next_white() * white;
                }
                if hum > 0.0 {
                    noise += self.hum_phase.sin() * hum;
                }
                noise += self.maybe_crackle();
            }

            self.hum_phase += phase_inc;
            if self.hum_phase > 2.0 * PI {
                self.hum_phase -= 2.0 * PI;
            }

            *s += noise;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: NoiseParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.white_amount = p.white_amount.clamp(0.0, MAX_WHITE);
        p.hum_amount = p.hum_amount.clamp(0.0, MAX_HUM);
        p.hum_hz = p.hum_hz.clamp(MIN_HUM_HZ, MAX_HUM_HZ);
        p.crackle_rate = p.crackle_rate.clamp(0.0, MAX_CRACKLE_RATE);
        p.speech_threshold_db = p.speech_threshold_db.clamp(-90.0, 0.0);
        self.params = p;
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("noise params serialize")
    }

    fn reset(&mut self) {
        self.hum_phase = 0.0;
        self.envelope = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_params_is_identity() {
        let mut n = Noise::new(48000, NoiseParams::default());
        let mut buf = vec![0.1, 0.2, -0.3];
        n.process(&mut buf);
        // No noise added → output equals input.
        assert!((buf[0] - 0.1).abs() < 1e-5);
        assert!((buf[1] - 0.2).abs() < 1e-5);
        assert!((buf[2] - (-0.3)).abs() < 1e-5);
    }

    #[test]
    fn white_amount_adds_audible_noise() {
        let mut n = Noise::new(48000, NoiseParams {
            white_amount: 0.3,
            ..Default::default()
        });
        let mut buf = vec![0.0; 1000];
        n.process(&mut buf);
        let max = buf.iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(max > 0.05, "white noise should produce signal, max {}", max);
    }

    #[test]
    fn hum_produces_periodic_signal() {
        let mut n = Noise::new(48000, NoiseParams {
            hum_amount: 0.3,
            hum_hz: 60.0,
            ..Default::default()
        });
        let mut buf = vec![0.0; 4800]; // 100 ms @ 48 kHz
        n.process(&mut buf);
        // 60 Hz over 100 ms = 6 cycles. Count zero crossings — expect ~12 (2 per cycle).
        let mut crossings = 0;
        for w in buf.windows(2) {
            if w[0].signum() != w[1].signum() && w[0] != 0.0 {
                crossings += 1;
            }
        }
        assert!((10..=14).contains(&crossings), "expected ~12 crossings, got {}", crossings);
    }

    #[test]
    fn on_speech_gates_when_input_quiet() {
        let mut n = Noise::new(48000, NoiseParams {
            white_amount: 0.5,
            gate_mode: NoiseGateMode::OnSpeech,
            speech_threshold_db: -30.0,
            ..Default::default()
        });
        // All-zero input → noise should be gated off (after envelope decays).
        let mut buf = vec![0.0; 4800];
        n.process(&mut buf);
        // After envelope has settled, late samples should be zero.
        let tail_max = buf[3000..].iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(tail_max < 0.01, "gated noise should be silent in tail, got {}", tail_max);
    }

    #[test]
    fn params_clamp_white_to_safe_range() {
        let mut n = Noise::new(48000, NoiseParams::default());
        n.set_params(&serde_json::json!({ "whiteAmount": 10.0 })).unwrap();
        let p: NoiseParams = serde_json::from_value(n.get_params()).unwrap();
        assert_eq!(p.white_amount, MAX_WHITE);
    }
}
