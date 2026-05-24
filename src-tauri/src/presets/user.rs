//! File-based user preset store.
//!
//! Layout: `<config_dir>/presets/<sanitized-name>.json`.
//! Filenames are derived from preset name with non-alphanumeric chars stripped.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::schema::Preset;
use crate::settings::Settings;

pub fn presets_dir() -> Result<PathBuf> {
    Ok(Settings::config_dir()?.join("presets"))
}

fn ensure_dir() -> Result<PathBuf> {
    let d = presets_dir()?;
    fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
    Ok(d)
}

fn sanitize_filename(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if out.is_empty() {
        out.push_str("preset");
    }
    out
}

pub fn save(preset: &Preset) -> Result<PathBuf> {
    let dir = ensure_dir()?;
    let path = dir.join(format!("{}.json", sanitize_filename(&preset.name)));
    let json = preset.to_json_string()?;
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

pub fn list() -> Result<Vec<Preset>> {
    let dir = presets_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(text) => match Preset::from_json_str(&text) {
                Ok(p) => out.push(p),
                Err(e) => log::warn!("skipping malformed preset {}: {}", path.display(), e),
            },
            Err(e) => log::warn!("could not read {}: {}", path.display(), e),
        }
    }
    // Sort alphabetically by name.
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn load_named(name: &str) -> Result<Preset> {
    let dir = presets_dir()?;
    let path = dir.join(format!("{}.json", sanitize_filename(name)));
    let text = fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    Preset::from_json_str(&text)
}

pub fn delete(name: &str) -> Result<()> {
    let dir = presets_dir()?;
    let path = dir.join(format!("{}.json", sanitize_filename(name)));
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_non_alphanumeric() {
        assert_eq!(sanitize_filename("Hello World!"), "Hello_World_");
        assert_eq!(sanitize_filename("preset-1"), "preset-1");
        assert_eq!(sanitize_filename("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_filename(""), "preset");
    }
}
