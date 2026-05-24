//! JSON-serializable preset types.
//!
//! Lenience rules:
//! - Missing fields fall back to serde defaults.
//! - Unknown effect `type_` entries are dropped silently by the chain builder
//!   (handled in registry.rs::make_effect, not here).
//! - Unknown top-level fields ignored.
//! - The Limiter is NEVER present in a preset — the chain builder always appends one.

use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

pub const PRESET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    #[serde(default = "default_version")]
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub effects: Vec<EffectInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EffectInstance {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub params: Json,
}

fn default_version() -> u32 { PRESET_SCHEMA_VERSION }
fn default_enabled() -> bool { true }

impl Preset {
    pub fn from_json_str(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn to_json_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip_minimal_preset() {
        let p = Preset {
            schema_version: 1,
            name: "Test".into(),
            description: None,
            effects: vec![],
        };
        let s = p.to_json_string().unwrap();
        let back = Preset::from_json_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn round_trip_with_effects() {
        let p = Preset {
            schema_version: 1,
            name: "Telephone".into(),
            description: Some("Tinny".into()),
            effects: vec![
                EffectInstance { type_: "gain".into(), enabled: true, params: json!({ "gainDb": 4.0 }) },
                EffectInstance { type_: "bandpass".into(), enabled: true, params: json!({ "lowCutHz": 300, "highCutHz": 3400 }) },
            ],
        };
        let s = p.to_json_string().unwrap();
        let back = Preset::from_json_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let json = r#"{ "name": "Spare", "effects": [] }"#;
        let p = Preset::from_json_str(json).unwrap();
        assert_eq!(p.schema_version, PRESET_SCHEMA_VERSION);
        assert!(p.description.is_none());
    }

    #[test]
    fn unknown_top_level_field_ignored() {
        let json = r#"{ "name": "X", "effects": [], "futureField": 42 }"#;
        let p = Preset::from_json_str(json).unwrap();
        assert_eq!(p.name, "X");
    }

    #[test]
    fn effect_instance_defaults_to_enabled() {
        let json = r#"{ "type": "gain" }"#;
        let inst: EffectInstance = serde_json::from_str(json).unwrap();
        assert!(inst.enabled);
        assert_eq!(inst.type_, "gain");
    }

    #[test]
    fn bad_json_returns_error() {
        let result = Preset::from_json_str("not json at all");
        assert!(result.is_err());
    }
}
