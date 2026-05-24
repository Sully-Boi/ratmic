use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub input_device_id: Option<String>,
    #[serde(default)]
    pub output_device_id: Option<String>,
    #[serde(default)]
    pub monitor_enabled: bool,
    #[serde(default)]
    pub monitor_device_id: Option<String>,
    // Default true: hearing-safe fallback when field is missing from an old config.
    #[serde(default = "default_true")]
    pub safe_output_mode: bool,
    #[serde(default)]
    pub last_preset_name: Option<String>,
}

fn default_schema_version() -> u32 {
    SETTINGS_SCHEMA_VERSION
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            input_device_id: None,
            output_device_id: None,
            monitor_enabled: false,
            monitor_device_id: None,
            safe_output_mode: true,
            last_preset_name: None,
        }
    }
}

impl Settings {
    pub fn config_dir() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "RatMic", "RatMic")
            .context("could not determine config dir")?;
        Ok(dirs.config_dir().to_path_buf())
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let s: Self = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", path.display()))?;
        Ok(s)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .with_context(|| format!("creating {}", dir.display()))?;
        let path = Self::config_path()?;
        let text = serde_json::to_string_pretty(self)?;
        fs::write(&path, text)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_safe_output_mode_on() {
        let s = Settings::default();
        assert!(s.safe_output_mode);
        assert_eq!(s.schema_version, SETTINGS_SCHEMA_VERSION);
    }

    #[test]
    fn round_trip_via_json() {
        let s = Settings {
            schema_version: 1,
            input_device_id: Some("USB Microphone (Realtek)".to_string()),
            output_device_id: Some("CABLE Input (VB-Audio)".to_string()),
            monitor_enabled: true,
            monitor_device_id: Some("Headphones (Realtek)".to_string()),
            safe_output_mode: false,
            last_preset_name: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let json = r#"{ "schema_version": 1 }"#;
        let parsed: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.input_device_id, None);
        assert!(parsed.safe_output_mode);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{ "schema_version": 1, "future_field": "ignored" }"#;
        let parsed: Settings = serde_json::from_str::<Settings>(json).unwrap();
        assert_eq!(parsed, Settings::default());
    }

    #[test]
    fn last_preset_name_round_trips() {
        let s = Settings {
            schema_version: 1,
            input_device_id: None,
            output_device_id: None,
            monitor_enabled: false,
            monitor_device_id: None,
            safe_output_mode: true,
            last_preset_name: Some("Tin Can".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_preset_name.as_deref(), Some("Tin Can"));
    }
}
