//! cpal device discovery + persistent identifiers.
//!
//! cpal does not expose stable device UUIDs on Windows. We persist devices
//! by their friendly name, validated at start of each engine session.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub kind: DeviceKind,
    /// Persistent ID we use to re-select this device later. For now, equal to name.
    pub id: String,
    pub is_default: bool,
}

pub fn list_devices() -> Result<Vec<DeviceInfo>> {
    let host = cpal::default_host();
    let mut out = Vec::new();

    let default_in_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();
    let default_out_name = host
        .default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    for d in host.input_devices().context("listing input devices")? {
        let name = d.name().unwrap_or_else(|_| "<unknown>".into());
        out.push(DeviceInfo {
            id: name.clone(),
            is_default: name == default_in_name,
            name,
            kind: DeviceKind::Input,
        });
    }

    for d in host.output_devices().context("listing output devices")? {
        let name = d.name().unwrap_or_else(|_| "<unknown>".into());
        out.push(DeviceInfo {
            id: name.clone(),
            is_default: name == default_out_name,
            name,
            kind: DeviceKind::Output,
        });
    }

    Ok(out)
}

pub fn find_input_device(id: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    for d in host.input_devices()? {
        if d.name().map(|n| n == id).unwrap_or(false) {
            return Ok(d);
        }
    }
    Err(anyhow!("input device not found: {id}"))
}

pub fn find_output_device(id: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    for d in host.output_devices()? {
        if d.name().map(|n| n == id).unwrap_or(false) {
            return Ok(d);
        }
    }
    Err(anyhow!("output device not found: {id}"))
}

pub fn default_input() -> Option<cpal::Device> {
    cpal::default_host().default_input_device()
}

pub fn default_output() -> Option<cpal::Device> {
    cpal::default_host().default_output_device()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_devices_smoke() {
        // On any dev machine with audio hardware, we expect at least one device.
        // CI without audio will list nothing, which is also acceptable; just don't crash.
        let result = list_devices();
        assert!(result.is_ok(), "list_devices errored: {:?}", result);
    }
}
