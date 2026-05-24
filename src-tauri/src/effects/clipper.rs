//! Hard + soft clipper with pre-gain drive and post trim.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipperParams {
    #[serde(default = "default_drive")]
    pub drive: f32,
    #[serde(rename = "hardClip", default = "default_hard")]
    pub hard_clip: f32,
    #[serde(rename = "softClip", default = "default_soft")]
    pub soft_clip: f32,
    #[serde(rename = "outputTrimDb", default)]
    pub output_trim_db: f32,
}

fn default_drive() -> f32 { 1.0 }
fn default_hard() -> f32 { 1.0 }
fn default_soft() -> f32 { 0.0 }

impl Default for ClipperParams {
    fn default() -> Self {
        Self {
            drive: default_drive(),
            hard_clip: default_hard(),
            soft_clip: default_soft(),
            output_trim_db: 0.0,
        }
    }
}

const MIN_DRIVE: f32 = 1.0;
const MAX_DRIVE: f32 = 10.0;
const MIN_TRIM_DB: f32 = -24.0;
const MAX_TRIM_DB: f32 = 6.0;

pub struct Clipper {
    params: ClipperParams,
    trim_amp: f32,
}

impl Clipper {
    pub fn new(params: ClipperParams) -> Self {
        let trim_amp = 10.0_f32.powf(params.output_trim_db / 20.0);
        Self { params, trim_amp }
    }
}

impl Effect for Clipper {
    fn type_name(&self) -> &'static str { "clipper" }

    fn process(&mut self, buffer: &mut [f32]) {
        let drive = self.params.drive;
        let hard = self.params.hard_clip.clamp(0.0, 1.0);
        let soft = self.params.soft_clip.clamp(0.0, 1.0);
        let trim = self.trim_amp;
        for s in buffer {
            let mut x = *s * drive;
            if soft > 0.0 {
                let soft_x = x.tanh();
                x = x * (1.0 - soft) + soft_x * soft;
            }
            if hard < 1.0 {
                x = x.clamp(-hard, hard);
            }
            *s = x * trim;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: ClipperParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.drive = p.drive.clamp(MIN_DRIVE, MAX_DRIVE);
        p.hard_clip = p.hard_clip.clamp(0.0, 1.0);
        p.soft_clip = p.soft_clip.clamp(0.0, 1.0);
        p.output_trim_db = p.output_trim_db.clamp(MIN_TRIM_DB, MAX_TRIM_DB);
        self.trim_amp = 10.0_f32.powf(p.output_trim_db / 20.0);
        self.params = p;
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("clipper params serialize")
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_drive_no_clip_is_identity() {
        let mut c = Clipper::new(ClipperParams::default());
        let mut buf = vec![0.1, -0.3, 0.5, -0.7];
        let original = buf.clone();
        c.process(&mut buf);
        for (a, b) in buf.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn hard_clip_clamps_at_threshold() {
        let mut c = Clipper::new(ClipperParams {
            drive: 1.0,
            hard_clip: 0.5,
            soft_clip: 0.0,
            output_trim_db: 0.0,
        });
        let mut buf = vec![0.9, -0.9, 0.4, -0.4];
        c.process(&mut buf);
        assert_eq!(buf[0], 0.5);
        assert_eq!(buf[1], -0.5);
        assert_eq!(buf[2], 0.4);
        assert_eq!(buf[3], -0.4);
    }

    #[test]
    fn drive_amplifies_pre_clip() {
        let mut c = Clipper::new(ClipperParams {
            drive: 4.0,
            hard_clip: 1.0,
            soft_clip: 0.0,
            output_trim_db: 0.0,
        });
        let mut buf = vec![0.1];
        c.process(&mut buf);
        assert!((buf[0] - 0.4).abs() < 1e-5);
    }

    #[test]
    fn soft_clip_bends_loud_signal_below_unity() {
        // tanh(2.0) ≈ 0.964, full soft mix should bring +2.0 down to ~0.964.
        let mut c = Clipper::new(ClipperParams {
            drive: 1.0,
            hard_clip: 1.0,
            soft_clip: 1.0,
            output_trim_db: 0.0,
        });
        let mut buf = vec![2.0];
        c.process(&mut buf);
        assert!(buf[0] < 0.99, "soft clipped 2.0 should be < 0.99, got {}", buf[0]);
        assert!(buf[0] > 0.9, "soft clipped 2.0 should be > 0.9, got {}", buf[0]);
    }

    #[test]
    fn trim_attenuates_output() {
        let mut c = Clipper::new(ClipperParams {
            drive: 1.0,
            hard_clip: 1.0,
            soft_clip: 0.0,
            output_trim_db: -6.02,
        });
        let mut buf = vec![1.0];
        c.process(&mut buf);
        // -6 dB ≈ 0.5x amplitude.
        assert!((buf[0] - 0.5).abs() < 1e-2);
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut c = Clipper::new(ClipperParams::default());
        c.set_params(&serde_json::json!({
            "drive": 100.0,
            "hardClip": 2.0,
            "softClip": -1.0,
            "outputTrimDb": 50.0
        })).unwrap();
        let p: ClipperParams = serde_json::from_value(c.get_params()).unwrap();
        assert_eq!(p.drive, MAX_DRIVE);
        assert_eq!(p.hard_clip, 1.0);
        assert_eq!(p.soft_clip, 0.0);
        assert_eq!(p.output_trim_db, MAX_TRIM_DB);
    }
}
