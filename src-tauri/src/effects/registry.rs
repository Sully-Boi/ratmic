//! Factory: build a Box<dyn Effect> from a (type_name, params_json) pair.

use anyhow::{anyhow, Result};
use serde_json::Value as Json;

use super::bandpass::{Bandpass, BandpassParams};
use super::bitcrusher::{Bitcrusher, BitcrusherParams};
use super::clipper::{Clipper, ClipperParams};
use super::gain::{Gain, GainParams};
use super::limiter::{Limiter, LimiterParams};
use super::noise::{Noise, NoiseParams};
use super::noise_gate::{NoiseGate, NoiseGateParams};
use super::packet_loss::{PacketLoss, PacketLossParams};
use super::Effect;

/// Canonical list of effect type_names that presets may reference (excluding limiter,
/// which is added by the chain builder, never by user presets).
pub const KNOWN_TYPES: &[&str] = &[
    "gain", "bandpass", "bitcrusher", "clipper",
    "noise", "packetLoss", "noiseGate",
];

pub fn make_effect(type_name: &str, params: &Json, sample_rate: u32) -> Result<Box<dyn Effect>> {
    let mut effect: Box<dyn Effect> = match type_name {
        "gain"       => Box::new(Gain::new(GainParams::default())),
        "bandpass"   => Box::new(Bandpass::new(sample_rate, BandpassParams::default())),
        "bitcrusher" => Box::new(Bitcrusher::new(sample_rate, BitcrusherParams::default())),
        "clipper"    => Box::new(Clipper::new(ClipperParams::default())),
        "noise"      => Box::new(Noise::new(sample_rate, NoiseParams::default())),
        "packetLoss" => Box::new(PacketLoss::new(sample_rate, PacketLossParams::default())),
        "noiseGate"  => Box::new(NoiseGate::new(sample_rate, NoiseGateParams::default())),
        "limiter"    => Box::new(Limiter::new(sample_rate, LimiterParams::default())),
        other => return Err(anyhow!("unknown effect type: {}", other)),
    };
    effect.set_params(params)?;
    Ok(effect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn make_known_types() {
        for ty in KNOWN_TYPES {
            let e = make_effect(ty, &json!({}), 48000);
            assert!(e.is_ok(), "failed to make {}: {:?}", ty, e.err());
        }
    }

    #[test]
    fn unknown_type_errors() {
        let e = make_effect("notReal", &json!({}), 48000);
        assert!(e.is_err());
    }

    #[test]
    fn params_applied_at_construction() {
        let e = make_effect("gain", &json!({ "gainDb": 12.0 }), 48000).unwrap();
        let p = e.get_params();
        assert!((p["gainDb"].as_f64().unwrap() - 12.0).abs() < 1e-3);
    }

    #[test]
    fn out_of_range_params_clamp_not_error() {
        let e = make_effect("gain", &json!({ "gainDb": 999.0 }), 48000).unwrap();
        let p = e.get_params();
        let g = p["gainDb"].as_f64().unwrap();
        assert!(g <= 24.0, "gain should clamp to <=24, got {}", g);
    }
}
