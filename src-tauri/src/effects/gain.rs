//! Input gain effect: ±24 dB.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GainParams {
    #[serde(rename = "gainDb", default)]
    pub gain_db: f32,
}

impl Default for GainParams {
    fn default() -> Self {
        Self { gain_db: 0.0 }
    }
}

pub struct Gain {
    params: GainParams,
    amp: f32,
}

impl Gain {
    pub fn new(params: GainParams) -> Self {
        let amp = db_to_amp(params.gain_db);
        Self { params, amp }
    }
}

fn db_to_amp(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

const MIN_DB: f32 = -24.0;
const MAX_DB: f32 = 24.0;

impl Effect for Gain {
    fn type_name(&self) -> &'static str { "gain" }

    fn process(&mut self, buffer: &mut [f32]) {
        let amp = self.amp;
        for s in buffer {
            *s *= amp;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: GainParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.gain_db = p.gain_db.clamp(MIN_DB, MAX_DB);
        self.amp = db_to_amp(p.gain_db);
        self.params = p;
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("gain params serialize")
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_is_identity() {
        let mut g = Gain::new(GainParams { gain_db: 0.0 });
        let mut buf = vec![0.1, 0.2, -0.5];
        g.process(&mut buf);
        assert_eq!(buf, vec![0.1, 0.2, -0.5]);
    }

    #[test]
    fn six_db_doubles_amplitude_approximately() {
        let mut g = Gain::new(GainParams { gain_db: 6.02 });
        let mut buf = vec![0.5; 4];
        g.process(&mut buf);
        for s in &buf {
            assert!((*s - 1.0).abs() < 1e-2, "+6 dB ≈ 2x, got {}", s);
        }
    }

    #[test]
    fn params_clamp_at_extremes() {
        let mut g = Gain::new(GainParams::default());
        g.set_params(&serde_json::json!({ "gainDb": 100.0 })).unwrap();
        let out: GainParams = serde_json::from_value(g.get_params()).unwrap();
        assert_eq!(out.gain_db, MAX_DB);
        g.set_params(&serde_json::json!({ "gainDb": -100.0 })).unwrap();
        let out: GainParams = serde_json::from_value(g.get_params()).unwrap();
        assert_eq!(out.gain_db, MIN_DB);
    }

    #[test]
    fn params_round_trip_through_json() {
        let mut g = Gain::new(GainParams { gain_db: 3.5 });
        let json = g.get_params();
        let mut g2 = Gain::new(GainParams::default());
        g2.set_params(&json).unwrap();
        let json2 = g2.get_params();
        assert_eq!(json, json2);
    }
}
