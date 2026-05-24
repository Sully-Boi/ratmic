//! Bit-depth quantization + sample-and-hold downsampling.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcrusherParams {
    #[serde(rename = "bitDepth", default = "default_bits")]
    pub bit_depth: u32,
    #[serde(rename = "sampleRateHz", default = "default_sr")]
    pub sample_rate_hz: u32,
    #[serde(default = "default_mix")]
    pub mix: f32,
}

fn default_bits() -> u32 { 16 }
fn default_sr() -> u32 { 48_000 }
fn default_mix() -> f32 { 1.0 }

impl Default for BitcrusherParams {
    fn default() -> Self {
        Self {
            bit_depth: default_bits(),
            sample_rate_hz: default_sr(),
            mix: default_mix(),
        }
    }
}

const MIN_BITS: u32 = 1;
const MAX_BITS: u32 = 16;
const MIN_SR: u32 = 1_000;
const MAX_SR: u32 = 48_000;

pub struct Bitcrusher {
    params: BitcrusherParams,
    /// Engine internal SR — used to compute the hold ratio.
    internal_sr: u32,
    hold_samples: u32,
    counter: u32,
    held_value: f32,
}

impl Bitcrusher {
    pub fn new(internal_sr: u32, params: BitcrusherParams) -> Self {
        let mut b = Self {
            params,
            internal_sr,
            hold_samples: 1,
            counter: 0,
            held_value: 0.0,
        };
        b.recompute();
        b
    }

    fn recompute(&mut self) {
        let ratio = (self.internal_sr as f32 / self.params.sample_rate_hz as f32).max(1.0);
        self.hold_samples = ratio.round().max(1.0) as u32;
    }

    fn quantize(&self, x: f32) -> f32 {
        let bits = self.params.bit_depth;
        if bits >= 16 {
            return x;
        }
        let levels = (1_u32 << bits) as f32;
        let half = levels / 2.0;
        let q = (x * half).round() / half;
        q.clamp(-1.0, 1.0)
    }
}

impl Effect for Bitcrusher {
    fn type_name(&self) -> &'static str { "bitcrusher" }

    fn process(&mut self, buffer: &mut [f32]) {
        let mix = self.params.mix.clamp(0.0, 1.0);
        let hold = self.hold_samples;
        for s in buffer {
            let dry = *s;
            if self.counter == 0 {
                self.held_value = self.quantize(dry);
            }
            self.counter = (self.counter + 1) % hold;
            *s = dry * (1.0 - mix) + self.held_value * mix;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: BitcrusherParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.bit_depth = p.bit_depth.clamp(MIN_BITS, MAX_BITS);
        p.sample_rate_hz = p.sample_rate_hz.clamp(MIN_SR, MAX_SR);
        p.mix = p.mix.clamp(0.0, 1.0);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("bitcrusher params serialize")
    }

    fn reset(&mut self) {
        self.counter = 0;
        self.held_value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_quality_full_wet_is_near_identity() {
        let mut b = Bitcrusher::new(48000, BitcrusherParams::default());
        let mut buf = vec![0.1_f32, 0.2, -0.5, 0.7];
        b.process(&mut buf);
        let expected = [0.1, 0.2, -0.5, 0.7];
        for (a, e) in buf.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-5);
        }
    }

    #[test]
    fn one_bit_collapses_to_sign() {
        let mut b = Bitcrusher::new(48000, BitcrusherParams {
            bit_depth: 1,
            sample_rate_hz: 48000,
            mix: 1.0,
        });
        let mut buf = vec![0.3, -0.4, 0.9, -0.8];
        b.process(&mut buf);
        // 1-bit ≈ sign function (±1 or 0)
        for s in &buf {
            assert!(*s == -1.0 || *s == 0.0 || *s == 1.0, "got {}", s);
        }
    }

    #[test]
    fn dry_mix_is_unchanged() {
        let mut b = Bitcrusher::new(48000, BitcrusherParams {
            bit_depth: 4,
            sample_rate_hz: 8000,
            mix: 0.0,
        });
        let mut buf = vec![0.1, 0.2, -0.5];
        b.process(&mut buf);
        assert_eq!(buf, vec![0.1, 0.2, -0.5]);
    }

    #[test]
    fn downsample_holds_value() {
        // 48 kHz internal, target 24 kHz → hold every 2 samples.
        let mut b = Bitcrusher::new(48000, BitcrusherParams {
            bit_depth: 16,
            sample_rate_hz: 24000,
            mix: 1.0,
        });
        let mut buf = vec![0.1, 0.5, 0.9, 0.3];
        b.process(&mut buf);
        // Samples 0,1 → both hold 0.1. Samples 2,3 → both hold 0.9.
        assert!((buf[0] - 0.1).abs() < 1e-5);
        assert!((buf[1] - 0.1).abs() < 1e-5);
        assert!((buf[2] - 0.9).abs() < 1e-5);
        assert!((buf[3] - 0.9).abs() < 1e-5);
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut b = Bitcrusher::new(48000, BitcrusherParams::default());
        b.set_params(&serde_json::json!({
            "bitDepth": 100,
            "sampleRateHz": 9999999,
            "mix": 2.0
        })).unwrap();
        let p: BitcrusherParams = serde_json::from_value(b.get_params()).unwrap();
        assert_eq!(p.bit_depth, MAX_BITS);
        assert_eq!(p.sample_rate_hz, MAX_SR);
        assert_eq!(p.mix, 1.0);
    }
}
