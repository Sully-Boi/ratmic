# RatMic Presets Implementation Plan (Phase 4 — preset system + chain editing)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship 10 curated built-in presets + user save/load/delete + minimal manual chain editing (add/remove effects). Loading a preset atomically rebuilds the chain without stopping the engine.

**Architecture:** A small `make_effect(type_name, params)` registry turns a JSON effect instance into a `Box<dyn Effect>`. The audio engine grows a `replace_chain` method that swaps the inner `EffectChain` under the mutex. Presets live as JSON: built-ins are bundled via `include_str!`, user presets live at `%APPDATA%\RatMic\presets\*.json`. The Limiter is never serialized in a preset — the chain rebuild always appends a fresh Limiter at the end as a safety belt. Frontend gets a `PresetSidebar` with built-in / user sections and a save-as dialog; the existing `EffectChain` component grows Remove (×) buttons and an "Add Effect" dropdown.

**Tech Stack:** Same as prior plans. No new deps.

**User preference:** No `git init`, no commits.

## Scope decisions (be ruthless)

| In | Out |
|---|---|
| 10 built-in presets | Custom preset categories / tags |
| Load preset (click) | Drag-to-reorder effects |
| Save current chain as new preset (name dialog) | Move Up / Move Down reorder buttons (cut: edit JSON on disk if needed) |
| Delete user preset (× button) | Import / export to file via dialogs (cut: just edit JSON files in the presets dir) |
| Duplicate preset (load → save under new name) | "Reset preset" UI action (cut: re-click the preset in sidebar) |
| Add effect (dropdown + Add button) | Per-effect wet/dry controls |
| Remove effect (× button on each row except Limiter) | Preset thumbnails / previews |
| Last-used preset remembered in settings | Preset versioning beyond schema_version field |

## Known Deferrals (from foundation/effects plans, still deferred)

- Internal-SR resampling via rubato (Plan 5).
- Routing health-check UI, Safe Output Mode parameter clamps, device-disconnect handling (Plan 5).
- Global hotkey, local monitor stream, 5-sec test record (Plan 3).

---

## File Structure Map

| Task | File |
|---|---|
| 1 | `src-tauri/src/effects/registry.rs` (new) |
| 1 | `src-tauri/src/effects/mod.rs` (register `registry`) |
| 2 | `src-tauri/src/presets/mod.rs` (new), `src-tauri/src/presets/schema.rs` (new) |
| 2 | `src-tauri/src/lib.rs` (register `presets`) |
| 3 | `src-tauri/src/audio/engine.rs` (add `replace_chain`, `add_effect_at_end`, `remove_effect_at`) |
| 3 | `src-tauri/src/effects/chain.rs` (add `replace_slots_from`, `remove`, expose `is_limiter` helper) |
| 4 | `src-tauri/src/presets/user.rs` (new) |
| 5 | `src-tauri/src/presets/builtin.rs` (new), `src-tauri/src/presets/builtin_json/*.json` (10 new) |
| 6 | `src-tauri/src/commands.rs` (add preset + chain-edit commands) |
| 6 | `src-tauri/src/lib.rs` (register new commands) |
| 6 | `src-tauri/src/settings.rs` (add `last_preset_name` field) |
| 7 | `src/lib/ipc.ts` (add preset & chain-edit methods + types) |
| 7 | `src/lib/stores.ts` (add `presets`, `userPresets` stores) |
| 8 | `src/lib/components/PresetSidebar.svelte` (new) |
| 8 | `src/lib/components/SavePresetDialog.svelte` (new) |
| 9 | `src/lib/components/EffectChain.svelte` (× buttons + Add Effect dropdown) |
| 9 | `src/App.svelte` (mount `PresetSidebar` in left pane) |
| 10 | Manual smoke test |

---

## Task 1: Effect factory registry

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\registry.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

Maps a `type_name` string + serde Json params to a fully-initialized `Box<dyn Effect>`. Single place where the supported set of effect types is enumerated.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod registry;` (alphabetical order).

- [ ] **Step 2: Implement registry with tests**

Create `src-tauri/src/effects/registry.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::registry`
Expected: 4 tests pass.

---

## Task 2: Preset schema

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\mod.rs`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\schema.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\lib.rs` (add `mod presets;`)

- [ ] **Step 1: Add module to lib.rs**

Edit `src-tauri/src/lib.rs`, add `mod presets;` next to the existing `mod audio; mod commands; mod effects; mod events; mod settings;` (alphabetical).

- [ ] **Step 2: Create presets module root**

Create `src-tauri/src/presets/mod.rs`:

```rust
pub mod schema;
```

(More modules added in later tasks.)

- [ ] **Step 3: Implement preset schema with tests**

Create `src-tauri/src/presets/schema.rs`:

```rust
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
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test presets::schema`
Expected: 6 tests pass.

---

## Task 3: Engine `replace_chain` + chain editing primitives

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\chain.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\engine.rs`

Atomic chain swap (used by preset load) + individual add/remove operations (used by chain-edit UI).

- [ ] **Step 1: Extend `EffectChain` with replace + remove**

Edit `src-tauri/src/effects/chain.rs`. Add the following methods inside `impl EffectChain` (preserving existing `new`, `push`, `len`, `is_empty`, `set_enabled`, `process`, `clear`, `slots_view`, `set_params`):

```rust
/// Replace all non-limiter slots with the given list, then append a fresh Limiter
/// at the end (always enabled). The Limiter is fixed by design and never
/// participates in a preset.
pub fn rebuild_from_slots(
    &mut self,
    sample_rate: u32,
    slots: Vec<(Box<dyn Effect>, bool)>,
) {
    use super::limiter::{Limiter, LimiterParams};
    self.slots.clear();
    for (effect, enabled) in slots {
        self.push(effect, enabled);
    }
    let limiter = Box::new(Limiter::new(sample_rate, LimiterParams::default()));
    self.push(limiter, true);
}

/// Insert an effect just before the final Limiter slot.
/// If there's no Limiter (shouldn't happen post-rebuild), appends to the end.
pub fn insert_before_limiter(&mut self, effect: Box<dyn Effect>, enabled: bool) {
    let pos = self.slots.iter().rposition(|s| s.effect.type_name() == "limiter");
    let slot = EffectSlot::new(effect, enabled, self.sample_rate);
    match pos {
        Some(idx) => self.slots.insert(idx, slot),
        None => self.slots.push(slot),
    }
}

/// Remove the slot at `index`. Refuses to remove the Limiter (returns false).
pub fn remove(&mut self, index: usize) -> bool {
    if let Some(slot) = self.slots.get(index) {
        if slot.effect.type_name() == "limiter" {
            return false;
        }
    } else {
        return false;
    }
    self.slots.remove(index);
    true
}
```

You also need to expose the `EffectSlot::new` constructor publicly. Find `impl EffectSlot { ... pub fn new(...) ... }` — it should already be `pub`. If not, make it `pub`.

- [ ] **Step 2: Add a test for chain rebuild + remove**

Append to `src-tauri/src/effects/chain.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn rebuild_appends_limiter_automatically() {
        use crate::effects::gain::{Gain, GainParams};
        use crate::effects::limiter::Limiter;
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(
            48000,
            vec![(Box::new(Gain::new(GainParams::default())), true)],
        );
        assert_eq!(c.len(), 2);
        let view = c.slots_view();
        assert_eq!(view[0].0, "gain");
        assert_eq!(view[1].0, "limiter");
        assert!(view[1].1, "limiter should be enabled");
    }

    #[test]
    fn rebuild_with_empty_just_has_limiter() {
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![]);
        assert_eq!(c.len(), 1);
        assert_eq!(c.slots_view()[0].0, "limiter");
    }

    #[test]
    fn remove_refuses_to_remove_limiter() {
        use crate::effects::gain::{Gain, GainParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![(Box::new(Gain::new(GainParams::default())), true)]);
        // Try to remove the limiter (index 1).
        let removed = c.remove(1);
        assert!(!removed, "should refuse to remove limiter");
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn remove_drops_non_limiter_slot() {
        use crate::effects::gain::{Gain, GainParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![(Box::new(Gain::new(GainParams::default())), true)]);
        let removed = c.remove(0);
        assert!(removed);
        assert_eq!(c.len(), 1);
        assert_eq!(c.slots_view()[0].0, "limiter");
    }

    #[test]
    fn insert_before_limiter_keeps_limiter_last() {
        use crate::effects::gain::{Gain, GainParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![]); // just limiter
        c.insert_before_limiter(Box::new(Gain::new(GainParams::default())), false);
        let view = c.slots_view();
        assert_eq!(view.len(), 2);
        assert_eq!(view[0].0, "gain");
        assert_eq!(view[1].0, "limiter");
    }
```

- [ ] **Step 3: Run chain tests**

Run: `cd src-tauri && cargo test effects::chain`
Expected: 4 prior chain tests + 5 new = 9 pass.

- [ ] **Step 4: Update engine to use rebuild + expose mutation API**

Edit `src-tauri/src/audio/engine.rs`. The `AudioEngine::start` already builds the default chain using `chain.push(...)` 8 times. Change the default startup to use `rebuild_from_slots` for consistency, and add three public methods.

Replace the default-chain construction block in `AudioEngine::start` (the 8 `chain.push(...)` calls and the trailing `let chain = Arc::new(Mutex::new(chain));`) with this:

```rust
        // Default chain on startup: empty (just the auto-appended Limiter).
        // A preset load — or manual Add Effect — populates it.
        let mut chain = EffectChain::new(INTERNAL_SAMPLE_RATE);
        chain.rebuild_from_slots(INTERNAL_SAMPLE_RATE, vec![]);
        let chain = Arc::new(Mutex::new(chain));
```

Then delete the now-unused individual effect imports at the top of `engine.rs`. The chain construction no longer needs Gain/Bandpass/Bitcrusher/Clipper/Noise/PacketLoss/NoiseGate/Limiter direct imports — only `EffectChain` is used here. Replace the 9 effect imports with just:

```rust
use crate::effects::chain::EffectChain;
```

Then add three methods to the existing `impl AudioEngine` block (place after `pub fn stop(...)`):

```rust
    /// Atomically replace the chain (used by preset load).
    pub fn replace_chain(
        &self,
        effect_specs: Vec<(String, bool, serde_json::Value)>,
    ) -> anyhow::Result<()> {
        let mut new_slots: Vec<(Box<dyn crate::effects::Effect>, bool)> = Vec::new();
        for (type_name, enabled, params) in effect_specs {
            match crate::effects::registry::make_effect(&type_name, &params, INTERNAL_SAMPLE_RATE) {
                Ok(e) => new_slots.push((e, enabled)),
                Err(err) => log::warn!("skipping unknown effect '{}': {}", type_name, err),
            }
        }
        let mut guard = self.chain.lock();
        guard.rebuild_from_slots(INTERNAL_SAMPLE_RATE, new_slots);
        Ok(())
    }

    /// Add a fresh effect of the given type to the chain (inserted before limiter).
    pub fn add_effect(&self, type_name: &str, enabled: bool) -> anyhow::Result<()> {
        let effect = crate::effects::registry::make_effect(
            type_name,
            &serde_json::json!({}),
            INTERNAL_SAMPLE_RATE,
        )?;
        let mut guard = self.chain.lock();
        guard.insert_before_limiter(effect, enabled);
        Ok(())
    }

    /// Remove the slot at the given index. Returns true on success, false if
    /// the index is the Limiter or out of range.
    pub fn remove_effect(&self, index: usize) -> bool {
        let mut guard = self.chain.lock();
        guard.remove(index)
    }
```

- [ ] **Step 5: Verify it compiles + all tests still pass**

Run: `cd src-tauri && cargo build`
Expected: clean (will produce dead-code warnings on Gain/Bandpass/etc imports if not removed; ensure step 4's import replacement was complete).

Run: `cd src-tauri && cargo test`
Expected: 73 prior + 4 registry + 6 schema + 5 chain (new) = **88 tests pass**.

---

## Task 4: User preset persistence

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\user.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\presets\mod.rs`

Files-on-disk store at `%APPDATA%\RatMic\RatMic\presets\<name>.json`. List/save/delete/load_named operations.

- [ ] **Step 1: Add module declaration**

Edit `src-tauri/src/presets/mod.rs`:

```rust
pub mod schema;
pub mod user;
```

- [ ] **Step 2: Implement user preset store**

Create `src-tauri/src/presets/user.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test presets::user`
Expected: 1 test passes. The full integration (save → list → load) is harder to unit-test without touching the real filesystem; that's verified manually in Task 10.

---

## Task 5: 10 built-in presets

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin.rs`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\xbox-360-lobby.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\cheap-headset.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\drive-thru-speaker.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\broken-radio.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\discord-packet-loss.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\deep-fried-mic.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\tin-can.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\underwater.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\fan-noise-hell.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\presets\builtin_json\2007-skype-call.json`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\presets\mod.rs`

Each preset is a JSON file bundled at compile time via `include_str!`. The Limiter is intentionally absent — the chain rebuild appends it.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/presets/mod.rs`:

```rust
pub mod builtin;
pub mod schema;
pub mod user;
```

- [ ] **Step 2: Create the 10 JSON files**

Create `src-tauri/src/presets/builtin_json/xbox-360-lobby.json`:

```json
{
  "schema_version": 1,
  "name": "Xbox 360 Lobby",
  "description": "Compressed, clipped, noisy old voice chat.",
  "effects": [
    { "type": "gain",       "enabled": true, "params": { "gainDb": 8 } },
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 300, "highCutHz": 3400, "midBoostDb": 4 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 8, "sampleRateHz": 11025, "mix": 0.8 } },
    { "type": "clipper",    "enabled": true, "params": { "drive": 2.5, "hardClip": 0.65, "softClip": 0.3, "outputTrimDb": -4 } },
    { "type": "packetLoss", "enabled": true, "params": { "dropChance": 0.08, "minDropMs": 30, "maxDropMs": 140, "stutterChance": 0.05 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/cheap-headset.json`:

```json
{
  "schema_version": 1,
  "name": "Cheap Headset",
  "description": "Tinny, slightly noisy, mildly distorted.",
  "effects": [
    { "type": "bandpass", "enabled": true, "params": { "lowCutHz": 200, "highCutHz": 4000, "midBoostDb": 2 } },
    { "type": "clipper",  "enabled": true, "params": { "drive": 1.5, "hardClip": 0.85, "softClip": 0.2, "outputTrimDb": -2 } },
    { "type": "noise",    "enabled": true, "params": { "whiteAmount": 0.04, "humAmount": 0, "crackleRate": 0 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/drive-thru-speaker.json`:

```json
{
  "schema_version": 1,
  "name": "Drive-Thru Speaker",
  "description": "Honky midrange, blown speaker crackle.",
  "effects": [
    { "type": "gain",     "enabled": true, "params": { "gainDb": 4 } },
    { "type": "bandpass", "enabled": true, "params": { "lowCutHz": 400, "highCutHz": 2500, "midBoostDb": 6 } },
    { "type": "clipper",  "enabled": true, "params": { "drive": 3.0, "hardClip": 0.55, "softClip": 0.4, "outputTrimDb": -3 } },
    { "type": "noise",    "enabled": true, "params": { "whiteAmount": 0.04, "humAmount": 0, "crackleRate": 5 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/broken-radio.json`:

```json
{
  "schema_version": 1,
  "name": "Broken Radio",
  "description": "Lo-fi band-limited, crackling static.",
  "effects": [
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 500, "highCutHz": 3500, "midBoostDb": -4 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 10, "sampleRateHz": 16000, "mix": 0.7 } },
    { "type": "noise",      "enabled": true, "params": { "whiteAmount": 0.08, "humAmount": 0, "crackleRate": 8 } },
    { "type": "packetLoss", "enabled": true, "params": { "dropChance": 0.1, "minDropMs": 40, "maxDropMs": 120, "stutterChance": 0 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/discord-packet-loss.json`:

```json
{
  "schema_version": 1,
  "name": "Discord Packet Loss",
  "description": "Choppy connection with stuttery replays.",
  "effects": [
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 100, "highCutHz": 8000, "midBoostDb": 0 } },
    { "type": "packetLoss", "enabled": true, "params": { "dropChance": 0.18, "minDropMs": 40, "maxDropMs": 180, "stutterChance": 0.15 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/deep-fried-mic.json`:

```json
{
  "schema_version": 1,
  "name": "Deep Fried Mic",
  "description": "Maximum overdrive. Limiter does heavy lifting.",
  "effects": [
    { "type": "gain",       "enabled": true, "params": { "gainDb": 18 } },
    { "type": "clipper",    "enabled": true, "params": { "drive": 8.0, "hardClip": 0.4, "softClip": 0.6, "outputTrimDb": -4 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 4, "sampleRateHz": 11025, "mix": 0.9 } },
    { "type": "noise",      "enabled": true, "params": { "whiteAmount": 0.05, "humAmount": 0, "crackleRate": 2 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/tin-can.json`:

```json
{
  "schema_version": 1,
  "name": "Tin Can",
  "description": "Narrow nasal resonance, like a string telephone.",
  "effects": [
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 800, "highCutHz": 2000, "midBoostDb": 10 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 10, "sampleRateHz": 22050, "mix": 0.5 } },
    { "type": "clipper",    "enabled": true, "params": { "drive": 2.0, "hardClip": 0.7, "softClip": 0.3, "outputTrimDb": -3 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/underwater.json`:

```json
{
  "schema_version": 1,
  "name": "Underwater",
  "description": "Muffled, dark, no highs.",
  "effects": [
    { "type": "bandpass", "enabled": true, "params": { "lowCutHz": 100, "highCutHz": 1500, "midBoostDb": -6 } },
    { "type": "noise",    "enabled": true, "params": { "whiteAmount": 0.03, "humAmount": 0, "crackleRate": 0 } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/fan-noise-hell.json`:

```json
{
  "schema_version": 1,
  "name": "Fan Noise Hell",
  "description": "Constant whir, hum, occasional click.",
  "effects": [
    { "type": "noise", "enabled": true, "params": { "whiteAmount": 0.15, "humAmount": 0.1, "humHz": 60, "crackleRate": 3, "gateMode": "always" } }
  ]
}
```

Create `src-tauri/src/presets/builtin_json/2007-skype-call.json`:

```json
{
  "schema_version": 1,
  "name": "2007 Skype Call",
  "description": "Narrowband, choppy, slightly clipped.",
  "effects": [
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 250, "highCutHz": 3000, "midBoostDb": 2 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 8, "sampleRateHz": 8000, "mix": 0.6 } },
    { "type": "clipper",    "enabled": true, "params": { "drive": 1.8, "hardClip": 0.8, "softClip": 0.2, "outputTrimDb": -3 } },
    { "type": "packetLoss", "enabled": true, "params": { "dropChance": 0.05, "minDropMs": 30, "maxDropMs": 100, "stutterChance": 0.1 } }
  ]
}
```

- [ ] **Step 3: Implement the loader**

Create `src-tauri/src/presets/builtin.rs`:

```rust
//! Built-in presets bundled at compile time.

use super::schema::Preset;

const RAW_PRESETS: &[(&str, &str)] = &[
    ("Xbox 360 Lobby",       include_str!("builtin_json/xbox-360-lobby.json")),
    ("Cheap Headset",        include_str!("builtin_json/cheap-headset.json")),
    ("Drive-Thru Speaker",   include_str!("builtin_json/drive-thru-speaker.json")),
    ("Broken Radio",         include_str!("builtin_json/broken-radio.json")),
    ("Discord Packet Loss",  include_str!("builtin_json/discord-packet-loss.json")),
    ("Deep Fried Mic",       include_str!("builtin_json/deep-fried-mic.json")),
    ("Tin Can",              include_str!("builtin_json/tin-can.json")),
    ("Underwater",           include_str!("builtin_json/underwater.json")),
    ("Fan Noise Hell",       include_str!("builtin_json/fan-noise-hell.json")),
    ("2007 Skype Call",      include_str!("builtin_json/2007-skype-call.json")),
];

pub fn all() -> Vec<Preset> {
    let mut out = Vec::with_capacity(RAW_PRESETS.len());
    for (label, raw) in RAW_PRESETS {
        match Preset::from_json_str(raw) {
            Ok(p) => out.push(p),
            Err(e) => panic!(
                "built-in preset '{}' failed to parse at compile time: {}",
                label, e
            ),
        }
    }
    out
}

pub fn by_name(name: &str) -> Option<Preset> {
    all().into_iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_built_in_presets_parse() {
        let presets = all();
        assert_eq!(presets.len(), 10, "expected 10 built-in presets");
    }

    #[test]
    fn each_preset_has_a_name() {
        for p in all() {
            assert!(!p.name.is_empty(), "preset missing name");
        }
    }

    #[test]
    fn xbox_360_lobby_round_trips() {
        let p = by_name("Xbox 360 Lobby").expect("Xbox 360 Lobby present");
        assert_eq!(p.effects.len(), 5);
        let types: Vec<&str> = p.effects.iter().map(|e| e.type_.as_str()).collect();
        assert_eq!(types, vec!["gain", "bandpass", "bitcrusher", "clipper", "packetLoss"]);
    }

    #[test]
    fn no_preset_contains_a_limiter_entry() {
        for p in all() {
            for e in &p.effects {
                assert_ne!(
                    e.type_, "limiter",
                    "preset '{}' must not list a limiter — chain builder appends one",
                    p.name
                );
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cd src-tauri && cargo test presets::builtin`
Expected: 4 tests pass.

---

## Task 6: Settings + Tauri commands

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\settings.rs` (add `last_preset_name`)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\commands.rs` (add preset + chain-edit commands)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\lib.rs` (register new commands)

- [ ] **Step 1: Add `last_preset_name` to settings**

Edit `src-tauri/src/settings.rs`. Add the new field to the `Settings` struct between `safe_output_mode` and the close brace:

```rust
    #[serde(default)]
    pub last_preset_name: Option<String>,
```

And add it to the `Default` impl:

```rust
            last_preset_name: None,
```

The existing settings tests should still pass since the field uses `#[serde(default)]`.

- [ ] **Step 2: Add an updated round-trip test for the new field**

Append to `src-tauri/src/settings.rs::tests`:

```rust
    #[test]
    fn last_preset_name_round_trips() {
        let s = Settings {
            schema_version: 1,
            input_device_id: None,
            output_device_id: None,
            monitor_enabled: false,
            safe_output_mode: true,
            last_preset_name: Some("Tin Can".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.last_preset_name.as_deref(), Some("Tin Can"));
    }
```

- [ ] **Step 3: Append preset + chain-edit commands to `commands.rs`**

Add to the imports at the top of `src-tauri/src/commands.rs`:

```rust
use crate::presets::builtin;
use crate::presets::schema::{EffectInstance, Preset};
use crate::presets::user;
```

Append at the end of the file:

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct PresetSummary {
    pub name: String,
    pub description: Option<String>,
    pub builtin: bool,
}

#[tauri::command]
pub fn list_presets() -> Result<Vec<PresetSummary>, String> {
    let mut out: Vec<PresetSummary> = builtin::all()
        .into_iter()
        .map(|p| PresetSummary { name: p.name, description: p.description, builtin: true })
        .collect();
    let users = user::list().map_err(|e| e.to_string())?;
    for p in users {
        out.push(PresetSummary { name: p.name, description: p.description, builtin: false });
    }
    Ok(out)
}

#[tauri::command]
pub fn load_preset(
    state: State<'_, AppState>,
    name: String,
    builtin_pref: bool,
) -> Result<(), String> {
    let preset = if builtin_pref {
        builtin::by_name(&name).ok_or_else(|| format!("built-in preset '{}' not found", name))?
    } else {
        user::load_named(&name).map_err(|e| e.to_string())?
    };

    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };

    let specs: Vec<(String, bool, Json)> = preset
        .effects
        .into_iter()
        .map(|e| (e.type_, e.enabled, e.params))
        .collect();
    engine.replace_chain(specs).map_err(|e| e.to_string())?;

    // Persist last-used preset.
    let mut settings = Settings::load().map_err(|e| e.to_string())?;
    settings.last_preset_name = Some(name);
    settings.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn save_preset_from_chain(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err("preset name cannot be empty".into());
    }
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    let chain = engine.chain.lock();
    let effects: Vec<EffectInstance> = chain
        .slots_view()
        .into_iter()
        // Strip the fixed Limiter — it's reapplied on load.
        .filter(|(type_name, _, _)| *type_name != "limiter")
        .map(|(type_name, enabled, params)| EffectInstance {
            type_: type_name.into(),
            enabled,
            params,
        })
        .collect();
    drop(chain);
    drop(guard);
    let preset = Preset {
        schema_version: crate::presets::schema::PRESET_SCHEMA_VERSION,
        name: trimmed,
        description,
        effects,
    };
    user::save(&preset).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_user_preset(name: String) -> Result<(), String> {
    user::delete(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_effect(
    state: State<'_, AppState>,
    type_name: String,
) -> Result<(), String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    engine.add_effect(&type_name, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_effect(state: State<'_, AppState>, index: usize) -> Result<bool, String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    Ok(engine.remove_effect(index))
}
```

- [ ] **Step 4: Register commands in `lib.rs`**

Edit `src-tauri/src/lib.rs`. Add the new commands to the `generate_handler!` macro (keeping existing commands intact):

```rust
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::list_audio_devices,
            commands::start_engine,
            commands::stop_engine,
            commands::engine_running,
            commands::get_chain,
            commands::set_effect_enabled,
            commands::set_effect_params,
            commands::list_presets,
            commands::load_preset,
            commands::save_preset_from_chain,
            commands::delete_user_preset,
            commands::add_effect,
            commands::remove_effect,
        ])
```

- [ ] **Step 5: Verify everything compiles and tests pass**

Run: `cd src-tauri && cargo build`
Expected: clean.

Run: `cd src-tauri && cargo test`
Expected: prior 88 + 1 new settings test = **89 tests pass**.

---

## Task 7: ipc.ts + stores

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\lib\ipc.ts`
- Modify: `E:\ClaudeCode\ratmic\src\lib\stores.ts`

- [ ] **Step 1: Add preset types + commands to `ipc.ts`**

Open `src/lib/ipc.ts`. Add at the top (after existing interfaces):

```ts
export interface PresetSummary {
  name: string;
  description: string | null;
  builtin: boolean;
}
```

Update the `Settings` interface to include the new field:

```ts
export interface Settings {
  schema_version: number;
  input_device_id: string | null;
  output_device_id: string | null;
  monitor_enabled: boolean;
  safe_output_mode: boolean;
  last_preset_name: string | null;
}
```

Extend the `ipc` const with the new methods. Replace the existing `ipc` block entirely with this version (keeps all existing entries plus the new ones):

```ts
export const ipc = {
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listDevices: () => invoke<DeviceInfo[]>("list_audio_devices"),
  startEngine: (inputId: string, outputId: string) =>
    invoke<void>("start_engine", { inputId, outputId }),
  stopEngine: () => invoke<void>("stop_engine"),
  engineRunning: () => invoke<boolean>("engine_running"),
  getChain: () => invoke<ChainSlotView[]>("get_chain"),
  setEffectEnabled: (index: number, enabled: boolean) =>
    invoke<void>("set_effect_enabled", { index, enabled }),
  setEffectParams: (index: number, params: unknown) =>
    invoke<void>("set_effect_params", { index, params }),
  listPresets: () => invoke<PresetSummary[]>("list_presets"),
  loadPreset: (name: string, builtinPref: boolean) =>
    invoke<void>("load_preset", { name, builtinPref }),
  savePresetFromChain: (name: string, description: string | null) =>
    invoke<void>("save_preset_from_chain", { name, description }),
  deleteUserPreset: (name: string) =>
    invoke<void>("delete_user_preset", { name }),
  addEffect: (typeName: string) =>
    invoke<void>("add_effect", { typeName }),
  removeEffect: (index: number) =>
    invoke<boolean>("remove_effect", { index }),
};
```

- [ ] **Step 2: Add `presets` store**

Edit `src/lib/stores.ts`. Add at the bottom:

```ts
import type { PresetSummary } from "./ipc";

export const presets = writable<PresetSummary[]>([]);
```

- [ ] **Step 3: Verify TypeScript checks**

Run: `npm run check`
Expected: 0 errors.

---

## Task 8: Preset sidebar UI

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\PresetSidebar.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\SavePresetDialog.svelte`
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte` (mount `PresetSidebar` in left pane)

- [ ] **Step 1: Build the SavePresetDialog**

Create `src/lib/components/SavePresetDialog.svelte`:

```svelte
<script lang="ts">
  export let open: boolean;
  export let onSave: (name: string, description: string) => void;
  export let onCancel: () => void;

  let name = "";
  let description = "";

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter" && name.trim()) onSave(name.trim(), description.trim());
  }

  $: if (open) {
    name = "";
    description = "";
  }
</script>

{#if open}
  <div class="backdrop" on:click={onCancel} on:keydown={handleKey} role="dialog" tabindex="-1">
    <div class="dialog" on:click|stopPropagation role="document">
      <h3>Save as preset</h3>
      <label>
        Name
        <input type="text" bind:value={name} placeholder="My Cursed Mic" autofocus />
      </label>
      <label>
        Description (optional)
        <input type="text" bind:value={description} placeholder="One-line description" />
      </label>
      <div class="row">
        <button on:click={onCancel}>Cancel</button>
        <button
          class="primary"
          disabled={!name.trim()}
          on:click={() => onSave(name.trim(), description.trim())}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: grid;
    place-items: center;
    z-index: 10;
  }
  .dialog {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  h3 { margin: 0 0 0.25rem; font-size: 13px; color: var(--text-1); }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 12px;
    color: var(--text-1);
  }
  input[type="text"] { width: 100%; }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
</style>
```

- [ ] **Step 2: Build the PresetSidebar**

Create `src/lib/components/PresetSidebar.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../ipc";
  import { presets, chain, engineRunning } from "../stores";
  import SavePresetDialog from "./SavePresetDialog.svelte";

  let saveOpen = false;
  let busy = false;
  let error = "";

  async function refresh() {
    try {
      const list = await ipc.listPresets();
      presets.set(list);
    } catch (e) {
      error = String(e);
    }
  }

  async function load(name: string, builtin: boolean) {
    if (!$engineRunning) {
      error = "start the engine first";
      return;
    }
    busy = true;
    error = "";
    try {
      await ipc.loadPreset(name, builtin);
      chain.set(await ipc.getChain());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function del(name: string) {
    busy = true;
    error = "";
    try {
      await ipc.deleteUserPreset(name);
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function onSave(name: string, description: string) {
    saveOpen = false;
    busy = true;
    error = "";
    try {
      await ipc.savePresetFromChain(name, description || null);
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  $: builtIns = $presets.filter((p) => p.builtin);
  $: userPresets = $presets.filter((p) => !p.builtin);

  onMount(refresh);
</script>

<h3>Presets</h3>

<button
  class="primary save-btn"
  on:click={() => (saveOpen = true)}
  disabled={!$engineRunning || busy}
>
  + Save current as preset
</button>

{#if error}
  <p class="err">{error}</p>
{/if}

<h4>Built-in</h4>
<ul class="list">
  {#each builtIns as p}
    <li>
      <button class="item" on:click={() => load(p.name, true)} title={p.description ?? ""}>
        {p.name}
      </button>
    </li>
  {/each}
</ul>

<h4>Your presets</h4>
{#if userPresets.length === 0}
  <p class="muted">(none yet)</p>
{:else}
  <ul class="list">
    {#each userPresets as p}
      <li class="row">
        <button class="item" on:click={() => load(p.name, false)} title={p.description ?? ""}>
          {p.name}
        </button>
        <button class="x" title="delete" on:click={() => del(p.name)}>×</button>
      </li>
    {/each}
  </ul>
{/if}

<SavePresetDialog
  open={saveOpen}
  onSave={onSave}
  onCancel={() => (saveOpen = false)}
/>

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  h4 {
    margin: 0.75rem 0 0.25rem;
    font-size: 11px;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .save-btn { width: 100%; margin-bottom: 0.5rem; font-size: 12px; }
  .err { color: var(--danger); font-size: 11px; margin: 0.25rem 0; }
  .muted { color: var(--text-2); font-size: 12px; margin: 0; }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .row {
    display: flex;
    gap: 0.25rem;
  }
  .item {
    flex: 1;
    text-align: left;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-0);
    padding: 0.35rem 0.5rem;
    border-radius: 4px;
    font: inherit;
    cursor: pointer;
  }
  .item:hover { background: var(--bg-3); }
  .x {
    width: 24px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-1);
    border-radius: 4px;
    cursor: pointer;
  }
  .x:hover { background: var(--danger); color: white; }
</style>
```

- [ ] **Step 3: Mount in App.svelte**

Open `src/App.svelte`. The left sidebar currently contains:

```svelte
<aside class="sidebar"><h3>Presets</h3><p class="muted">(coming soon)</p></aside>
```

Replace with:

```svelte
<aside class="sidebar"><PresetSidebar /></aside>
```

Add the import at the top of the `<script>` block (with the other component imports):

```ts
import PresetSidebar from "./lib/components/PresetSidebar.svelte";
```

- [ ] **Step 4: Verify TypeScript + visual sanity**

Run: `npm run check`
Expected: 0 errors.

(Visual verification deferred to Task 10.)

---

## Task 9: Chain UI editing (Remove + Add Effect)

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\lib\components\EffectChain.svelte`

- [ ] **Step 1: Replace EffectChain.svelte with the editing-capable version**

Replace `src/lib/components/EffectChain.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../ipc";
  import { chain, selectedEffectIndex, engineRunning } from "../stores";

  const ADDABLE_TYPES = [
    "gain", "bandpass", "bitcrusher", "clipper",
    "noise", "packetLoss", "noiseGate",
  ];

  let addType = "gain";
  let busy = false;

  async function refresh() {
    try {
      chain.set(await ipc.getChain());
    } catch (e) {
      console.error(e);
    }
  }

  async function toggle(index: number, enabled: boolean) {
    try {
      await ipc.setEffectEnabled(index, enabled);
      await refresh();
    } catch (e) {
      console.error(e);
    }
  }

  async function remove(index: number) {
    busy = true;
    try {
      await ipc.removeEffect(index);
      // If the removed slot was selected, clear selection.
      if ($selectedEffectIndex === index) selectedEffectIndex.set(null);
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      busy = false;
    }
  }

  async function add() {
    busy = true;
    try {
      await ipc.addEffect(addType);
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      busy = false;
    }
  }

  $: if ($engineRunning) refresh();

  onMount(refresh);
</script>

<h3>Effect Chain</h3>
<ul class="list">
  {#each $chain as slot}
    <li
      class:selected={$selectedEffectIndex === slot.index}
      class:fixed={slot.type_name === "limiter"}
      on:click={() => selectedEffectIndex.set(slot.index)}
      on:keydown={(e) => {
        if (e.key === "Enter" || e.key === " ") selectedEffectIndex.set(slot.index);
      }}
      role="button"
      tabindex="0"
    >
      <input
        type="checkbox"
        checked={slot.enabled}
        on:change={(e) => toggle(slot.index, (e.target as HTMLInputElement).checked)}
        disabled={!$engineRunning || slot.type_name === "limiter"}
        on:click|stopPropagation
      />
      <span class="name">{slot.type_name}</span>
      {#if slot.type_name === "limiter"}
        <span class="badge">fixed</span>
      {:else}
        <button
          class="x"
          title="remove"
          on:click|stopPropagation={() => remove(slot.index)}
          disabled={!$engineRunning || busy}
        >
          ×
        </button>
      {/if}
    </li>
  {/each}
</ul>

{#if $engineRunning}
  <div class="add-row">
    <select bind:value={addType} disabled={busy}>
      {#each ADDABLE_TYPES as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
    <button on:click={add} disabled={busy}>+ Add</button>
  </div>
{:else}
  <p class="muted">Start the engine to edit the chain.</p>
{/if}

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }
  li:hover { background: var(--bg-3); }
  li.selected { border-color: var(--accent); }
  li.fixed { opacity: 0.85; }
  .name {
    flex: 1;
    text-transform: capitalize;
  }
  .badge {
    font-size: 10px;
    color: var(--text-2);
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .x {
    width: 24px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-1);
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 2px 0;
  }
  .x:hover { background: var(--danger); color: white; }
  .add-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  select { flex: 1; }
  .muted { color: var(--text-2); font-size: 12px; margin-top: 0.5rem; }
</style>
```

- [ ] **Step 2: Verify TypeScript**

Run: `npm run check`
Expected: 0 errors.

---

## Task 10: Manual smoke test

After the implementer dispatch reports DONE, the user verifies end-to-end.

- [ ] **Step 1: Launch the app**

Run: `npm run tauri dev`
Expected: window opens. Left pane shows "Presets" with a `+ Save current as preset` button (disabled until engine starts), "Built-in" list with 10 entries, "Your presets" with "(none yet)".

- [ ] **Step 2: Verify default chain is just Limiter**

Effect Chain pane shows only one row: `limiter` (with "fixed" badge). No × button. There's also a dropdown + "+ Add" button at the bottom (greyed out until engine starts).

- [ ] **Step 3: Start the engine + load a built-in preset**

Pick input/output devices, click START. Click "Xbox 360 Lobby" in the sidebar. Effect Chain should refresh to show 6 rows: gain, bandpass, bitcrusher, clipper, packetLoss, limiter. Speak — voice should sound noticeably Xbox-y.

- [ ] **Step 4: Cycle through built-ins**

Click each of the 10 built-ins in turn. Each should change the chain and the audible character. No errors in console.

- [ ] **Step 5: Save a user preset**

While "Tin Can" is loaded, click `+ Save current as preset`. Type "My Tin Can" in the dialog, click Save. The user list should now show "My Tin Can". Click it — chain should reload identically.

- [ ] **Step 6: Verify file on disk**

Open `%APPDATA%\RatMic\RatMic\presets\` in File Explorer. There should be `My_Tin_Can.json` containing the same effects as the Tin Can preset (no Limiter listed).

- [ ] **Step 7: Delete the user preset**

In the sidebar, hover "My Tin Can" and click the × button. It disappears from the list. Refresh the directory in File Explorer — the JSON file should be gone.

- [ ] **Step 8: Add and remove effects manually**

Load "Underwater" preset (2 effects + limiter). In the chain UI, click the × next to "noise" → it disappears. Pick "clipper" in the dropdown, click "+ Add" → clipper appears just before limiter. Voice should reflect both changes.

- [ ] **Step 9: Limiter is protected**

Verify the limiter row has no × button and its checkbox stays disabled. The chain always ends with a limiter, no matter what preset loads or what you add/remove.

- [ ] **Step 10: Persistence across restart**

Note the currently-loaded preset, close the app, relaunch. Devices should still be remembered. (The `last_preset_name` field is saved but auto-restoring it on launch is deferred — the user can re-click their preset.)

- [ ] **Step 11: All Rust tests still pass**

Run: `cd src-tauri && cargo test`
Expected: 89/89 pass.

- [ ] **Step 12: TypeScript clean**

Run: `npm run check`
Expected: 0 errors.

---

## Final verification

When all 10 tasks are checked off:

- [ ] App launches, sidebar lists 10 built-in presets + (initially empty) user list.
- [ ] Clicking a built-in preset rebuilds the chain and changes the sound.
- [ ] Saving the current chain as a user preset persists to `%APPDATA%\RatMic\RatMic\presets\<name>.json`.
- [ ] Deleting a user preset removes the file and the list entry.
- [ ] Manual Add Effect adds a slot just before the Limiter.
- [ ] Manual Remove removes any non-Limiter slot.
- [ ] Limiter is always present at the end and cannot be removed or disabled.
- [ ] 89 Rust tests pass, `npm run check` clean.
- [ ] All built-in presets sound noticeably different from each other on default speech.

Plan 5 (routing health + Safe Mode + resampling) and Plan 3 (hotkey + monitor + test record) remain to be planned.
