# RatMic Foundation Implementation Plan (Phases 0–2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land a working Tauri+Svelte app on Windows that captures audio from a chosen input device, runs it through a modular effect chain containing a Gain effect and a fixed final limiter, and outputs to a chosen output device — with input/output meters, click-free toggle, and ≤ 30 ms RTT.

**Architecture:** Tauri 2 Rust backend with cpal (WASAPI shared mode) audio I/O. A dedicated audio worker thread reads from a lock-free input ring buffer, runs the effect chain, and writes to an output ring buffer drained by the cpal output callback. Svelte frontend communicates via Tauri IPC commands and 60 Hz event-driven meter updates.

**Tech Stack:** Tauri 2.x, Rust (edition 2021), Svelte 4 + TypeScript, Vite. Audio: `cpal` 0.15+, `ringbuf` 0.4+, `dasp` 0.11+, `directories` 5+, `serde` 1, `serde_json` 1, `thiserror` 1, `anyhow` 1, `parking_lot` 0.12+.

**User preference:** No `git init`, no commits during the plan unless explicitly asked. The "Verify" steps replace commit checkpoints.

## Known Deferrals (in scope for later plans, not this one)

- **Internal-SR resampling.** The spec calls for a fixed 48 kHz internal pipeline with `rubato` resampling at I/O boundaries. This plan does **not** wire resamplers — the worker thread runs at whatever sample rate the input device produces, and the output backend expects the same SR. For Phase 1 smoke testing this is fine on common modern setups (most USB mics + VB-CABLE are 48 kHz). The next plan adds proper resampling. Task 1.5 emits a `log::warn!` if input/output SR mismatch is detected.
- **Local monitor stream, test recording, hotkey, presets, full effect library.** All Phase 3+ scope.
- **Routing health check, device-disconnect handling, Safe Output Mode clamps.** Phase 6 scope.

---

## File Structure Map

Files created during this plan, grouped by task:

### Phase 0 — Skeleton
| Task | File |
|---|---|
| 0.1 | `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`, `.gitignore`, `README.md` |
| 0.1 | `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs` |
| 0.2 | `src-tauri/Cargo.toml` (deps added) |
| 0.3 | `src-tauri/src/settings.rs` |
| 0.4 | `src-tauri/src/commands.rs`, `src-tauri/src/main.rs` (wire commands) |
| 0.5 | `src/App.svelte`, `src/app.css` (dark theme) |

### Phase 1 — Audio Passthrough
| Task | File |
|---|---|
| 1.1 | `src-tauri/src/audio/mod.rs`, `src-tauri/src/audio/format.rs` |
| 1.2 | `src-tauri/src/audio/ring_buffer.rs` |
| 1.3 | `src-tauri/src/audio/meters.rs` |
| 1.4 | `src-tauri/src/audio/devices.rs` |
| 1.5 | `src-tauri/src/audio/output_backend.rs`, `src-tauri/src/audio/system_output.rs` |
| 1.6 | `src-tauri/src/audio/input_stream.rs` |
| 1.7 | `src-tauri/src/audio/engine.rs` |
| 1.8 | `src-tauri/src/events.rs`, `src-tauri/src/commands.rs` (audio commands) |
| 1.9 | `src/lib/ipc.ts`, `src/lib/stores.ts`, `src/lib/components/DeviceBar.svelte` |
| 1.10 | `src/lib/components/MeterBar.svelte` |
| 1.11 | `src/App.svelte` (wire engine controls) |

### Phase 2 — Effect Chain Framework
| Task | File |
|---|---|
| 2.1 | `src-tauri/src/effects/mod.rs` |
| 2.2 | `src-tauri/src/effects/crossfade.rs` |
| 2.3 | `src-tauri/src/effects/chain.rs` |
| 2.4 | `src-tauri/src/effects/gain.rs` |
| 2.5 | `src-tauri/src/effects/limiter.rs` |
| 2.6 | `src-tauri/src/audio/engine.rs` (wire chain into worker) |
| 2.7 | `src-tauri/src/commands.rs` (chain commands) |
| 2.8 | `src/lib/components/EffectChain.svelte` |
| 2.9 | `src/lib/components/EffectParams.svelte` |

---

## Phase 0 — Skeleton

### Task 0.1: Scaffold Tauri 2 + Svelte 4 + TypeScript project

**Files:**
- Create: `E:\ClaudeCode\ratmic\package.json`
- Create: `E:\ClaudeCode\ratmic\tsconfig.json`
- Create: `E:\ClaudeCode\ratmic\vite.config.ts`
- Create: `E:\ClaudeCode\ratmic\index.html`
- Create: `E:\ClaudeCode\ratmic\src\main.ts`
- Create: `E:\ClaudeCode\ratmic\src\App.svelte`
- Create: `E:\ClaudeCode\ratmic\src\app.css`
- Create: `E:\ClaudeCode\ratmic\.gitignore`
- Create: `E:\ClaudeCode\ratmic\README.md`
- Create: `E:\ClaudeCode\ratmic\src-tauri\Cargo.toml`
- Create: `E:\ClaudeCode\ratmic\src-tauri\tauri.conf.json`
- Create: `E:\ClaudeCode\ratmic\src-tauri\build.rs`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Verify Rust toolchain is installed**

Run: `rustc --version`
Expected: `rustc 1.75.0` or newer. If missing, install from https://rustup.rs.

Run: `cargo --version`
Expected: `cargo 1.75.0` or newer.

Run: `node --version`
Expected: `v20.x.x` or newer. If missing, install from https://nodejs.org.

- [ ] **Step 2: Create `package.json`**

```json
{
  "name": "ratmic",
  "private": true,
  "version": "0.0.1",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "check": "svelte-check --tsconfig ./tsconfig.json"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.0.0",
    "@tauri-apps/cli": "^2.0.0",
    "@tsconfig/svelte": "^5.0.0",
    "svelte": "^4.2.0",
    "svelte-check": "^3.6.0",
    "tslib": "^2.6.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0"
  }
}
```

- [ ] **Step 3: Create `tsconfig.json`**

```json
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "baseUrl": ".",
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleResolution": "Bundler",
    "strict": true,
    "noImplicitAny": true
  },
  "include": ["src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 4: Create `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 5: Create `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [svelte({ preprocess: vitePreprocess() })],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

(`vitePreprocess()` is required to strip TypeScript from `<script lang="ts">` blocks at runtime — `svelte-check` uses its own TS-aware parser and doesn't depend on this, which is why a missing preprocessor passes lint but fails at dev-server boot.)

- [ ] **Step 6: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>RatMic</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 7: Create `src/main.ts`**

```ts
import "./app.css";
import App from "./App.svelte";

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
```

- [ ] **Step 8: Create `src/App.svelte` (placeholder)**

```svelte
<script lang="ts">
  let appName = "RatMic";
</script>

<main>
  <h1>{appName}</h1>
  <p>Audio engine initializing...</p>
</main>

<style>
  main {
    padding: 1rem;
  }
</style>
```

- [ ] **Step 9: Create `src/app.css` (placeholder, dark theme expanded in 0.5)**

```css
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  font-size: 14px;
  color-scheme: dark;
}

body {
  margin: 0;
  background: #1a1a1a;
  color: #e8e8e8;
}
```

- [ ] **Step 10: Create `.gitignore`**

```
node_modules/
dist/
src-tauri/target/
*.log
.env
.DS_Store
```

- [ ] **Step 11: Create `README.md`**

```markdown
# RatMic

A Windows desktop app that makes your microphone sound intentionally bad in real time.

## Dev

```bash
npm install
npm run tauri dev
```

See `docs/superpowers/specs/2026-05-24-ratmic-design.md` for the design spec.
```

- [ ] **Step 12: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "ratmic"
version = "0.0.1"
description = "Bad microphone simulator"
edition = "2021"
rust-version = "1.75"

[lib]
name = "ratmic_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

- [ ] **Step 13: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "RatMic",
  "version": "0.0.1",
  "identifier": "com.ratmic.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "RatMic",
        "width": 1100,
        "height": 720,
        "minWidth": 900,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.ico"]
  }
}
```

- [ ] **Step 14: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 14b: Create `src-tauri/capabilities/default.json`**

Tauri 2 gates the global event channel behind permissions. Custom `invoke` commands work without it (auto-allowed by `generate_handler!`), but `listen()` from `@tauri-apps/api/event` silently fails to receive events unless the `core:default` permission set is granted to the window.

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main RatMic window: allows event listen/emit so the frontend can subscribe to meter and engine-state events.",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 15: Create `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 16: Create a placeholder icon directory**

Run: `New-Item -ItemType Directory -Path "src-tauri\icons" -Force` (PowerShell) or `mkdir -p src-tauri/icons` (Bash).

For now create a placeholder by copying any 256×256 PNG named `icon.ico` into `src-tauri/icons/`. Tauri requires this file to exist for `tauri.conf.json` to validate. If no icon is available, generate one with: `npm run tauri icon -- placeholder.png` after step 17.

- [ ] **Step 17: Install npm dependencies**

Run: `npm install`
Expected: completes without errors, creates `node_modules/` and `package-lock.json`.

- [ ] **Step 18: Verify Rust side compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles cleanly, may take 2–5 min on first run while pulling tauri deps.

- [ ] **Step 19: Launch dev server (smoke test)**

Run: `npm run tauri dev`
Expected: Tauri window opens, shows "RatMic" heading and "Audio engine initializing..." text. Dev server URL `http://localhost:1420` reachable. Window can be closed.

If you hit "missing icon" error: regenerate icons with `npm run tauri icon path/to/any.png`.

---

### Task 0.2: Add core Rust dependencies

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\Cargo.toml`

- [ ] **Step 1: Add audio and utility crates to `[dependencies]`**

Replace the `[dependencies]` block in `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
parking_lot = "0.12"
directories = "5"
cpal = "0.15"
ringbuf = "0.4"
dasp = { version = "0.11", features = ["signal"] }
log = "0.4"
env_logger = "0.11"
```

- [ ] **Step 2: Verify dependencies resolve**

Run: `cd src-tauri && cargo check`
Expected: compiles cleanly. Warnings about unused imports are OK at this stage.

---

### Task 0.3: Settings struct with TDD

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\settings.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Write failing tests for Settings serialization**

Create `src-tauri/src/settings.rs`:

```rust
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
    #[serde(default = "default_true")]
    pub safe_output_mode: bool,
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
            safe_output_mode: true,
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
            safe_output_mode: false,
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
}
```

- [ ] **Step 2: Register `settings` module**

Edit `src-tauri/src/main.rs` to add module declaration:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test settings::`
Expected: 4 tests pass — `default_has_safe_output_mode_on`, `round_trip_via_json`, `missing_fields_use_defaults`, `unknown_fields_are_ignored`.

---

### Task 0.4: Settings IPC commands

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\commands.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Create commands module**

Create `src-tauri/src/commands.rs`:

```rust
use crate::settings::Settings;

#[tauri::command]
pub fn load_settings() -> Result<Settings, String> {
    Settings::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Register commands in `main.rs`**

Replace `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod settings;

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly. Note: `env_logger` was added in 0.2 deps; verify it's there.

- [ ] **Step 4: Smoke test settings round-trip from frontend**

Edit `src/App.svelte`:

```svelte
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let status = "loading...";

  onMount(async () => {
    try {
      const s = await invoke("load_settings");
      status = "loaded: " + JSON.stringify(s);
      await invoke("save_settings", { settings: s });
      status += " (saved back)";
    } catch (e) {
      status = "error: " + e;
    }
  });
</script>

<main>
  <h1>RatMic</h1>
  <p>{status}</p>
</main>

<style>
  main {
    padding: 1rem;
  }
</style>
```

- [ ] **Step 5: Run app and verify**

Run: `npm run tauri dev`
Expected: app launches, status shows `loaded: {...} (saved back)`. Check the file exists at `%APPDATA%\RatMic\RatMic\settings.json` via File Explorer.

---

### Task 0.5: Dark theme baseline UI

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\app.css`
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte`

- [ ] **Step 1: Expand global styles**

Replace `src/app.css`:

```css
:root {
  --bg-0: #0e0e10;
  --bg-1: #161618;
  --bg-2: #1f1f23;
  --bg-3: #2a2a2f;
  --border: #2f2f35;
  --text-0: #e8e8e8;
  --text-1: #a8a8ad;
  --text-2: #6a6a70;
  --accent: #d97706;
  --accent-hot: #fb923c;
  --danger: #dc2626;
  --warn: #d97706;
  --ok: #16a34a;

  font-family: "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif;
  font-size: 13px;
  color-scheme: dark;
}

* {
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  margin: 0;
}

body {
  background: var(--bg-0);
  color: var(--text-0);
  overflow: hidden;
  user-select: none;
}

button {
  background: var(--bg-2);
  color: var(--text-0);
  border: 1px solid var(--border);
  padding: 0.4rem 0.8rem;
  border-radius: 4px;
  font: inherit;
  cursor: pointer;
}

button:hover {
  background: var(--bg-3);
}

button.primary {
  background: var(--accent);
  border-color: var(--accent);
}

button.primary:hover {
  background: var(--accent-hot);
}

select, input[type="text"], input[type="number"] {
  background: var(--bg-1);
  color: var(--text-0);
  border: 1px solid var(--border);
  padding: 0.3rem 0.5rem;
  border-radius: 4px;
  font: inherit;
}
```

- [ ] **Step 2: Add layout shell to `App.svelte`**

Replace `src/App.svelte`:

```svelte
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let status = "loading...";

  onMount(async () => {
    try {
      const s = await invoke("load_settings");
      status = "settings loaded";
    } catch (e) {
      status = "error: " + e;
    }
  });
</script>

<div class="shell">
  <header class="top-bar">
    <strong>RatMic</strong>
    <span class="status">{status}</span>
  </header>
  <main class="body">
    <aside class="sidebar">Presets</aside>
    <section class="chain">Effect Chain</section>
    <aside class="params">Parameters</aside>
  </main>
  <footer class="bottom-bar">Meters · Start/Stop</footer>
</div>

<style>
  .shell {
    display: grid;
    grid-template-rows: 36px 1fr 56px;
    height: 100%;
    background: var(--bg-0);
  }
  .top-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 0.75rem;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
  }
  .status {
    color: var(--text-2);
    font-size: 12px;
  }
  .body {
    display: grid;
    grid-template-columns: 180px 1fr 280px;
    min-height: 0;
  }
  .sidebar, .chain, .params {
    padding: 0.75rem;
    overflow: auto;
  }
  .sidebar {
    background: var(--bg-1);
    border-right: 1px solid var(--border);
  }
  .params {
    background: var(--bg-1);
    border-left: 1px solid var(--border);
  }
  .bottom-bar {
    display: flex;
    align-items: center;
    padding: 0 0.75rem;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
  }
</style>
```

- [ ] **Step 3: Verify layout**

Run: `npm run tauri dev`
Expected: app shows three-pane dark layout, top status bar reads "settings loaded", bottom bar visible.

---

### Task 0.6: Phase 0 smoke test

- [ ] **Step 1: Verify file exists**

Open File Explorer to `%APPDATA%\RatMic\RatMic\settings.json`. File should exist with default values.

- [ ] **Step 2: Verify cargo tests still pass**

Run: `cd src-tauri && cargo test`
Expected: all settings tests pass.

- [ ] **Step 3: Verify TypeScript checks pass**

Run: `npm run check`
Expected: 0 errors, 0 warnings.

---

## Phase 1 — Audio Passthrough

### Task 1.1: Audio format helpers (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\format.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Create audio module skeleton**

Create `src-tauri/src/audio/mod.rs`:

```rust
pub mod format;
```

Edit `src-tauri/src/main.rs` to add `mod audio;` next to existing modules.

- [ ] **Step 2: Write failing tests for format helpers**

Create `src-tauri/src/audio/format.rs`:

```rust
//! Sample format conversion + simple downmix helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Downmix interleaved multi-channel f32 samples to mono in-place.
/// Returns the number of mono samples written.
pub fn downmix_to_mono(input: &[f32], channels: u16, out: &mut Vec<f32>) -> usize {
    out.clear();
    if channels <= 1 {
        out.extend_from_slice(input);
        return input.len();
    }
    let ch = channels as usize;
    let frames = input.len() / ch;
    out.reserve(frames);
    for f in 0..frames {
        let mut sum = 0.0;
        for c in 0..ch {
            sum += input[f * ch + c];
        }
        out.push(sum / ch as f32);
    }
    frames
}

/// Upmix mono samples to interleaved n-channel by duplicating each sample.
pub fn mono_to_interleaved(input: &[f32], channels: u16, out: &mut Vec<f32>) {
    out.clear();
    let ch = channels as usize;
    out.reserve(input.len() * ch);
    for &s in input {
        for _ in 0..ch {
            out.push(s);
        }
    }
}

/// Convert i16 PCM to f32 in [-1.0, 1.0].
pub fn i16_to_f32(input: &[i16], out: &mut Vec<f32>) {
    out.clear();
    out.reserve(input.len());
    let scale = 1.0 / 32768.0_f32;
    for &s in input {
        out.push(s as f32 * scale);
    }
}

/// Convert f32 in [-1.0, 1.0] to i16 PCM with hard clipping.
pub fn f32_to_i16(input: &[f32], out: &mut Vec<i16>) {
    out.clear();
    out.reserve(input.len());
    for &s in input {
        let clipped = s.clamp(-1.0, 1.0);
        out.push((clipped * 32767.0) as i16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_passthrough() {
        let mut out = Vec::new();
        let n = downmix_to_mono(&[0.1, 0.2, 0.3], 1, &mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn stereo_downmix_averages_channels() {
        let mut out = Vec::new();
        let n = downmix_to_mono(&[1.0, -1.0, 0.5, -0.5], 2, &mut out);
        assert_eq!(n, 2);
        assert!((out[0]).abs() < 1e-6);
        assert!((out[1]).abs() < 1e-6);
    }

    #[test]
    fn upmix_duplicates() {
        let mut out = Vec::new();
        mono_to_interleaved(&[0.1, 0.2], 2, &mut out);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
    }

    #[test]
    fn i16_round_trip_near_unity() {
        let original_f = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let mut as_i16: Vec<i16> = Vec::new();
        f32_to_i16(&original_f, &mut as_i16);
        let mut back_f = Vec::new();
        i16_to_f32(&as_i16, &mut back_f);
        for (a, b) in original_f.iter().zip(back_f.iter()) {
            assert!((a - b).abs() < 1e-3, "round-trip drift: {} vs {}", a, b);
        }
    }

    #[test]
    fn f32_to_i16_clamps_extremes() {
        let mut out = Vec::new();
        f32_to_i16(&[2.0, -2.0], &mut out);
        assert_eq!(out, vec![32767, -32767]);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test audio::format`
Expected: 5 tests pass.

---

### Task 1.2: Lock-free ring buffer wrapper (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\ring_buffer.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add module declaration**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod format;
pub mod ring_buffer;
```

- [ ] **Step 2: Write failing tests + implementation**

Create `src-tauri/src/audio/ring_buffer.rs`:

```rust
//! Typed SPSC ring buffer wrapper over `ringbuf`.

use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

pub struct AudioRing {
    capacity: usize,
}

pub struct RingProducer {
    inner: <HeapRb<f32> as Split>::Prod,
}

pub struct RingConsumer {
    inner: <HeapRb<f32> as Split>::Cons,
}

impl AudioRing {
    /// `capacity` is number of f32 samples the buffer can hold.
    pub fn new(capacity: usize) -> (RingProducer, RingConsumer) {
        let rb = HeapRb::<f32>::new(capacity);
        let (prod, cons) = rb.split();
        (RingProducer { inner: prod }, RingConsumer { inner: cons })
    }
}

impl RingProducer {
    /// Push as many samples as fit. Returns count written.
    pub fn push(&mut self, src: &[f32]) -> usize {
        self.inner.push_slice(src)
    }

    pub fn free_len(&self) -> usize {
        self.inner.vacant_len()
    }
}

impl RingConsumer {
    /// Pop up to `dst.len()` samples into `dst`. Returns count read.
    pub fn pop(&mut self, dst: &mut [f32]) -> usize {
        self.inner.pop_slice(dst)
    }

    pub fn occupied_len(&self) -> usize {
        self.inner.occupied_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_then_pop_round_trips() {
        let (mut p, mut c) = AudioRing::new(16);
        let pushed = p.push(&[1.0, 2.0, 3.0]);
        assert_eq!(pushed, 3);
        let mut out = [0.0_f32; 4];
        let popped = c.pop(&mut out);
        assert_eq!(popped, 3);
        assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn push_caps_at_capacity() {
        let (mut p, mut _c) = AudioRing::new(4);
        let pushed = p.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(pushed, 4);
    }

    #[test]
    fn pop_caps_at_occupied() {
        let (mut p, mut c) = AudioRing::new(8);
        p.push(&[1.0, 2.0]);
        let mut out = [0.0_f32; 5];
        let popped = c.pop(&mut out);
        assert_eq!(popped, 2);
        assert_eq!(&out[..2], &[1.0, 2.0]);
    }

    #[test]
    fn occupied_and_free_track_state() {
        let (mut p, mut c) = AudioRing::new(8);
        assert_eq!(c.occupied_len(), 0);
        assert_eq!(p.free_len(), 8);
        p.push(&[1.0, 2.0, 3.0]);
        assert_eq!(c.occupied_len(), 3);
        assert_eq!(p.free_len(), 5);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test audio::ring_buffer`
Expected: 4 tests pass.

---

### Task 1.3: Peak/RMS meter (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\meters.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add module**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod format;
pub mod meters;
pub mod ring_buffer;
```

- [ ] **Step 2: Write meter with tests**

Create `src-tauri/src/audio/meters.rs`:

```rust
//! Peak + RMS meter with exponential decay.

#[derive(Debug, Clone, Copy)]
pub struct MeterValue {
    pub peak: f32,
    pub rms: f32,
}

impl MeterValue {
    pub const ZERO: Self = MeterValue { peak: 0.0, rms: 0.0 };

    pub fn peak_db(&self) -> f32 {
        amp_to_db(self.peak)
    }

    pub fn rms_db(&self) -> f32 {
        amp_to_db(self.rms)
    }
}

pub fn amp_to_db(amp: f32) -> f32 {
    if amp <= 1e-9 {
        -90.0
    } else {
        20.0 * amp.log10()
    }
}

pub struct Meter {
    peak: f32,
    rms_sum: f32,
    rms_count: u32,
    /// per-sample decay multiplier for peak.
    peak_decay: f32,
}

impl Meter {
    /// `sample_rate` is internal SR, `peak_release_ms` is time for peak to decay by ~99%.
    pub fn new(sample_rate: u32, peak_release_ms: f32) -> Self {
        let release_samples = (sample_rate as f32) * (peak_release_ms * 0.001).max(0.001);
        let peak_decay = (-1.0 / release_samples).exp();
        Self {
            peak: 0.0,
            rms_sum: 0.0,
            rms_count: 0,
            peak_decay,
        }
    }

    pub fn process(&mut self, samples: &[f32]) {
        for &s in samples {
            let abs = s.abs();
            if abs > self.peak {
                self.peak = abs;
            } else {
                self.peak *= self.peak_decay;
            }
            self.rms_sum += s * s;
            self.rms_count += 1;
        }
    }

    /// Drain accumulated samples into a MeterValue and reset the RMS window.
    pub fn snapshot(&mut self) -> MeterValue {
        let rms = if self.rms_count > 0 {
            (self.rms_sum / self.rms_count as f32).sqrt()
        } else {
            0.0
        };
        let value = MeterValue { peak: self.peak, rms };
        self.rms_sum = 0.0;
        self.rms_count = 0;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_signal_zero_meter() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.0; 480]);
        let v = m.snapshot();
        assert_eq!(v.peak, 0.0);
        assert_eq!(v.rms, 0.0);
    }

    #[test]
    fn unity_peak_detected() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[1.0]);
        let v = m.snapshot();
        assert!((v.peak - 1.0).abs() < 1e-6);
    }

    #[test]
    fn negative_peaks_use_absolute_value() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[-0.7, -0.2, 0.5]);
        let v = m.snapshot();
        assert!((v.peak - 0.7).abs() < 1e-6);
    }

    #[test]
    fn rms_of_constant_is_constant() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.5; 100]);
        let v = m.snapshot();
        assert!((v.rms - 0.5).abs() < 1e-6);
    }

    #[test]
    fn amp_to_db_anchor_values() {
        assert!((amp_to_db(1.0) - 0.0).abs() < 1e-6);
        assert!((amp_to_db(0.5) - (-6.020599)).abs() < 1e-3);
        assert_eq!(amp_to_db(0.0), -90.0);
    }

    #[test]
    fn snapshot_resets_rms() {
        let mut m = Meter::new(48000, 100.0);
        m.process(&[0.5; 100]);
        let _ = m.snapshot();
        m.process(&[0.0; 100]);
        let v = m.snapshot();
        assert!(v.rms < 1e-6);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test audio::meters`
Expected: 6 tests pass.

---

### Task 1.4: Device enumeration with persistent IDs

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\devices.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add module**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod devices;
pub mod format;
pub mod meters;
pub mod ring_buffer;
```

- [ ] **Step 2: Implement device enumeration**

Create `src-tauri/src/audio/devices.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test audio::devices`
Expected: 1 test passes.

- [ ] **Step 4: Quick visual check**

Run: `cd src-tauri && cargo run --bin ratmic` (skip if launch is slow). Or write a temporary `bin/list_devices.rs` if you want. The main smoke test is in Task 1.9.

---

### Task 1.5: Output backend trait + SystemDeviceBackend

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\output_backend.rs`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\system_output.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add modules**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod devices;
pub mod format;
pub mod meters;
pub mod output_backend;
pub mod ring_buffer;
pub mod system_output;
```

- [ ] **Step 2: Define output backend trait**

Create `src-tauri/src/audio/output_backend.rs`:

```rust
//! Abstraction over output destinations.
//!
//! v1: SystemDeviceBackend (cpal output stream).
//! v2: RatMicVirtualMicBackend (first-party virtual driver).
//!
//! The audio engine talks only to this trait; swapping backends does not affect
//! effects, presets, or UI.

use anyhow::Result;

use super::format::AudioFormat;

pub trait AudioOutputBackend: Send {
    fn name(&self) -> &str;
    fn open(&mut self, format: AudioFormat) -> Result<()>;
    /// Submit processed samples (interleaved if multi-channel).
    /// Returns the number of samples actually written.
    fn write(&mut self, samples: &[f32]) -> Result<usize>;
    fn close(&mut self);
}
```

- [ ] **Step 3: Implement SystemDeviceBackend**

Create `src-tauri/src/audio/system_output.rs`:

```rust
//! cpal-based system output device backend.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::format::{f32_to_i16, mono_to_interleaved, AudioFormat};
use super::output_backend::AudioOutputBackend;
use super::ring_buffer::{AudioRing, RingProducer};

pub struct SystemDeviceBackend {
    name: String,
    device: cpal::Device,
    stream: Option<Stream>,
    producer: Option<RingProducer>,
    format: Option<AudioFormat>,
    /// Output ring capacity: 4× a 512-sample buffer × max 2 channels = 4096 samples.
    ring_capacity: usize,
    /// Scratch for upmix.
    scratch: Vec<f32>,
}

impl SystemDeviceBackend {
    pub fn new(device: cpal::Device) -> Self {
        let name = device.name().unwrap_or_else(|_| "<unknown>".into());
        Self {
            name,
            device,
            stream: None,
            producer: None,
            format: None,
            ring_capacity: 4096,
            scratch: Vec::with_capacity(4096),
        }
    }
}

impl AudioOutputBackend for SystemDeviceBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&mut self, format: AudioFormat) -> Result<()> {
        let supported = self
            .device
            .default_output_config()
            .context("getting default output config")?;
        let actual_format = AudioFormat {
            sample_rate: supported.sample_rate().0,
            channels: supported.channels(),
        };
        if actual_format.sample_rate != format.sample_rate {
            log::warn!(
                "Output device SR ({}) differs from requested ({}); resampling will be added later. \
                 For Phase 1 pick devices that share SR.",
                actual_format.sample_rate,
                format.sample_rate
            );
        }
        let config: StreamConfig = supported.config();
        let channels = config.channels;

        let (producer, mut consumer) = AudioRing::new(self.ring_capacity);

        let err_fn = |e| log::error!("output stream error: {e}");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => self.device.build_output_stream(
                &config,
                move |out: &mut [f32], _| {
                    let n = consumer.pop(out);
                    if n < out.len() {
                        for s in &mut out[n..] {
                            *s = 0.0;
                        }
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => {
                let mut tmp: Vec<f32> = vec![0.0; 8192];
                self.device.build_output_stream(
                    &config,
                    move |out: &mut [i16], _| {
                        if tmp.len() < out.len() {
                            tmp.resize(out.len(), 0.0);
                        }
                        let n = consumer.pop(&mut tmp[..out.len()]);
                        let mut i16_buf = Vec::with_capacity(out.len());
                        f32_to_i16(&tmp[..n], &mut i16_buf);
                        for (i, s) in i16_buf.iter().enumerate() {
                            out[i] = *s;
                        }
                        for s in &mut out[n..] {
                            *s = 0;
                        }
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(anyhow!("unsupported output sample format: {:?}", other));
            }
        }
        .context("building output stream")?;

        stream.play().context("starting output stream")?;

        self.stream = Some(stream);
        self.producer = Some(producer);
        self.format = Some(AudioFormat {
            sample_rate: config.sample_rate.0,
            channels,
        });
        Ok(())
    }

    fn write(&mut self, samples: &[f32]) -> Result<usize> {
        let fmt = self
            .format
            .ok_or_else(|| anyhow!("backend not opened"))?;
        let producer = self
            .producer
            .as_mut()
            .ok_or_else(|| anyhow!("backend not opened"))?;
        if fmt.channels <= 1 {
            return Ok(producer.push(samples));
        }
        mono_to_interleaved(samples, fmt.channels, &mut self.scratch);
        Ok(producer.push(&self.scratch))
    }

    fn close(&mut self) {
        if let Some(s) = self.stream.take() {
            let _ = s.pause();
        }
        self.producer = None;
        self.format = None;
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly. Warnings about unused functions are OK.

---

### Task 1.6: Input stream wrapper

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\input_stream.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add module**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod devices;
pub mod format;
pub mod input_stream;
pub mod meters;
pub mod output_backend;
pub mod ring_buffer;
pub mod system_output;
```

- [ ] **Step 2: Implement input stream**

Create `src-tauri/src/audio/input_stream.rs`:

```rust
//! cpal-based input stream that drains samples into an output `RingProducer`.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::format::{downmix_to_mono, i16_to_f32, AudioFormat};
use super::ring_buffer::RingProducer;

pub struct InputStream {
    _stream: Stream,
    pub device_format: AudioFormat,
}

impl InputStream {
    pub fn open(
        device: &cpal::Device,
        mut out_producer: RingProducer,
    ) -> Result<Self> {
        let supported = device
            .default_input_config()
            .context("getting default input config")?;
        let device_format = AudioFormat {
            sample_rate: supported.sample_rate().0,
            channels: supported.channels(),
        };
        let config: StreamConfig = supported.config();
        let channels = config.channels;

        let err_fn = |e| log::error!("input stream error: {e}");

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let mut scratch_mono: Vec<f32> = Vec::with_capacity(2048);
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        downmix_to_mono(data, channels, &mut scratch_mono);
                        let _ = out_producer.push(&scratch_mono);
                    },
                    err_fn,
                    None,
                )
            }
            SampleFormat::I16 => {
                let mut scratch_f32: Vec<f32> = Vec::with_capacity(2048);
                let mut scratch_mono: Vec<f32> = Vec::with_capacity(2048);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        i16_to_f32(data, &mut scratch_f32);
                        downmix_to_mono(&scratch_f32, channels, &mut scratch_mono);
                        let _ = out_producer.push(&scratch_mono);
                    },
                    err_fn,
                    None,
                )
            }
            other => {
                return Err(anyhow!("unsupported input sample format: {:?}", other));
            }
        }
        .context("building input stream")?;

        stream.play().context("starting input stream")?;

        Ok(Self {
            _stream: stream,
            device_format,
        })
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly.

---

### Task 1.7: Audio engine + worker thread (identity passthrough)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\audio\engine.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\mod.rs`

- [ ] **Step 1: Add module**

Edit `src-tauri/src/audio/mod.rs`:

```rust
pub mod devices;
pub mod engine;
pub mod format;
pub mod input_stream;
pub mod meters;
pub mod output_backend;
pub mod ring_buffer;
pub mod system_output;
```

- [ ] **Step 2: Implement engine**

Create `src-tauri/src/audio/engine.rs`:

```rust
//! Top-level audio engine: owns input stream, worker thread, output backend.
//!
//! Phase 1: identity passthrough (no effects). Worker pulls mono samples from
//! input ring, runs meters, pushes to output backend.

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::devices::{find_input_device, find_output_device};
use super::format::AudioFormat;
use super::input_stream::InputStream;
use super::meters::{Meter, MeterValue};
use super::output_backend::AudioOutputBackend;
use super::ring_buffer::{AudioRing, RingConsumer};
use super::system_output::SystemDeviceBackend;

pub const INTERNAL_SAMPLE_RATE: u32 = 48_000;
const INPUT_RING_CAPACITY: usize = 8192;
const WORKER_CHUNK_SAMPLES: usize = 480; // 10 ms @ 48 kHz
const METER_TICK_MS: u64 = 16;

#[derive(Debug, Clone, Copy)]
pub struct MeterSnapshot {
    pub input: MeterValue,
    pub output: MeterValue,
}

pub trait MeterSink: Send + 'static {
    fn push(&self, snap: MeterSnapshot);
}

pub struct AudioEngine {
    _input: InputStream,
    worker_handle: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
}

impl AudioEngine {
    pub fn start<S: MeterSink + 'static>(
        input_id: &str,
        output_id: &str,
        meter_sink: S,
    ) -> Result<Self> {
        if input_id == output_id {
            return Err(anyhow!("input and output device must differ"));
        }
        let input_device = find_input_device(input_id)
            .with_context(|| format!("opening input device {input_id}"))?;
        let output_device = find_output_device(output_id)
            .with_context(|| format!("opening output device {output_id}"))?;

        // Build input ring + cpal input stream.
        let (in_prod, in_cons) = AudioRing::new(INPUT_RING_CAPACITY);
        let input_stream = InputStream::open(&input_device, in_prod)
            .context("opening input stream")?;

        // Build output backend.
        let mut backend = SystemDeviceBackend::new(output_device);
        backend
            .open(AudioFormat {
                sample_rate: INTERNAL_SAMPLE_RATE,
                channels: 1,
            })
            .context("opening output backend")?;
        let backend: Arc<Mutex<Box<dyn AudioOutputBackend>>> =
            Arc::new(Mutex::new(Box::new(backend)));

        // Spawn worker.
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            let backend = backend.clone();
            thread::Builder::new()
                .name("ratmic-audio-worker".into())
                .spawn(move || {
                    worker_loop(in_cons, backend, meter_sink, stop);
                })
                .context("spawning audio worker")?
        };

        log::info!(
            "audio engine started: in='{}' out='{}', device-in SR={}, internal SR={}",
            input_id,
            output_id,
            input_stream.device_format.sample_rate,
            INTERNAL_SAMPLE_RATE
        );

        Ok(Self {
            _input: input_stream,
            worker_handle: Some(worker),
            stop_flag: stop,
            backend,
        })
    }

    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        self.backend.lock().close();
        log::info!("audio engine stopped");
    }
}

fn worker_loop<S: MeterSink>(
    mut consumer: RingConsumer,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    sink: S,
    stop: Arc<AtomicBool>,
) {
    let mut buffer = vec![0.0_f32; WORKER_CHUNK_SAMPLES];
    let mut in_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let mut out_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let meter_interval = Duration::from_millis(METER_TICK_MS);
    let mut last_meter = std::time::Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let n = consumer.pop(&mut buffer);
        if n == 0 {
            // No input yet, sleep briefly.
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        let chunk = &mut buffer[..n];

        in_meter.process(chunk);
        // Phase 1: identity DSP. (Effect chain wired in Phase 2.)
        out_meter.process(chunk);

        let written = backend.lock().write(chunk).unwrap_or(0);
        if written < n {
            log::trace!("output backend dropped {} samples (full)", n - written);
        }

        if last_meter.elapsed() >= meter_interval {
            sink.push(MeterSnapshot {
                input: in_meter.snapshot(),
                output: out_meter.snapshot(),
            });
            last_meter = std::time::Instant::now();
        }
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly.

---

### Task 1.8: Audio IPC commands + meter events

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\events.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\commands.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Define event types**

Create `src-tauri/src/events.rs`:

```rust
use serde::Serialize;

pub const EVENT_METERS: &str = "meters";
pub const EVENT_ENGINE_STATE: &str = "engine-state";

#[derive(Debug, Clone, Serialize)]
pub struct MeterEvent {
    pub input_peak_db: f32,
    pub input_rms_db: f32,
    pub output_peak_db: f32,
    pub output_rms_db: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStateEvent {
    pub running: bool,
    pub error: Option<String>,
}
```

- [ ] **Step 2: Add audio commands and engine state**

Replace `src-tauri/src/commands.rs`:

```rust
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

use crate::audio::devices::{list_devices, DeviceInfo};
use crate::audio::engine::{AudioEngine, MeterSink, MeterSnapshot};
use crate::events::{EngineStateEvent, MeterEvent, EVENT_ENGINE_STATE, EVENT_METERS};
use crate::settings::Settings;

pub struct AppState {
    pub engine: Mutex<Option<AudioEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
        }
    }
}

struct EmitSink {
    app: AppHandle,
}

impl MeterSink for EmitSink {
    fn push(&self, snap: MeterSnapshot) {
        let ev = MeterEvent {
            input_peak_db: snap.input.peak_db(),
            input_rms_db: snap.input.rms_db(),
            output_peak_db: snap.output.peak_db(),
            output_rms_db: snap.output.rms_db(),
        };
        let _ = self.app.emit(EVENT_METERS, ev);
    }
}

#[tauri::command]
pub fn load_settings() -> Result<Settings, String> {
    Settings::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<DeviceInfo>, String> {
    list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_engine(
    app: AppHandle,
    state: State<'_, AppState>,
    input_id: String,
    output_id: String,
) -> Result<(), String> {
    let mut guard = state.engine.lock();
    if guard.is_some() {
        return Err("engine already running".into());
    }
    let sink = EmitSink { app: app.clone() };
    let engine = AudioEngine::start(&input_id, &output_id, sink)
        .map_err(|e| e.to_string())?;
    *guard = Some(engine);
    let _ = app.emit(
        EVENT_ENGINE_STATE,
        EngineStateEvent {
            running: true,
            error: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn stop_engine(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.engine.lock();
    if let Some(engine) = guard.take() {
        engine.stop();
    }
    let _ = app.emit(
        EVENT_ENGINE_STATE,
        EngineStateEvent {
            running: false,
            error: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn engine_running(state: State<'_, AppState>) -> bool {
    state.engine.lock().is_some()
}
```

- [ ] **Step 3: Wire state + commands in main.rs**

Replace `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod events;
mod settings;

use commands::AppState;

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::list_audio_devices,
            commands::start_engine,
            commands::stop_engine,
            commands::engine_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Verify compile**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly.

---

### Task 1.9: DeviceBar component + ipc.ts

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\ipc.ts`
- Create: `E:\ClaudeCode\ratmic\src\lib\stores.ts`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\DeviceBar.svelte`

- [ ] **Step 1: Typed IPC wrapper**

Create `src/lib/ipc.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DeviceKind = "Input" | "Output";

export interface DeviceInfo {
  name: string;
  kind: DeviceKind;
  id: string;
  is_default: boolean;
}

export interface Settings {
  schema_version: number;
  input_device_id: string | null;
  output_device_id: string | null;
  monitor_enabled: boolean;
  safe_output_mode: boolean;
}

export interface MeterEvent {
  input_peak_db: number;
  input_rms_db: number;
  output_peak_db: number;
  output_rms_db: number;
}

export interface EngineStateEvent {
  running: boolean;
  error: string | null;
}

export const ipc = {
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listDevices: () => invoke<DeviceInfo[]>("list_audio_devices"),
  startEngine: (inputId: string, outputId: string) =>
    invoke<void>("start_engine", { inputId, outputId }),
  stopEngine: () => invoke<void>("stop_engine"),
  engineRunning: () => invoke<boolean>("engine_running"),
};

export const events = {
  onMeters: (cb: (e: MeterEvent) => void): Promise<UnlistenFn> =>
    listen<MeterEvent>("meters", (e) => cb(e.payload)),
  onEngineState: (cb: (e: EngineStateEvent) => void): Promise<UnlistenFn> =>
    listen<EngineStateEvent>("engine-state", (e) => cb(e.payload)),
};
```

- [ ] **Step 2: Svelte stores**

Create `src/lib/stores.ts`:

```ts
import { writable } from "svelte/store";
import type { Settings, MeterEvent } from "./ipc";

export const settings = writable<Settings | null>(null);
export const inputDeviceId = writable<string | null>(null);
export const outputDeviceId = writable<string | null>(null);
export const engineRunning = writable<boolean>(false);
export const meters = writable<MeterEvent>({
  input_peak_db: -90,
  input_rms_db: -90,
  output_peak_db: -90,
  output_rms_db: -90,
});
export const engineError = writable<string | null>(null);
```

- [ ] **Step 3: DeviceBar component**

Create `src/lib/components/DeviceBar.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, type DeviceInfo } from "../ipc";
  import { inputDeviceId, outputDeviceId } from "../stores";

  let inputs: DeviceInfo[] = [];
  let outputs: DeviceInfo[] = [];
  let loadError = "";

  async function refresh() {
    try {
      const all = await ipc.listDevices();
      inputs = all.filter((d) => d.kind === "Input");
      outputs = all.filter((d) => d.kind === "Output");
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(refresh);
</script>

<div class="bar">
  <label>
    Input
    <select bind:value={$inputDeviceId}>
      <option value={null}>— select —</option>
      {#each inputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>
  <label>
    Output
    <select bind:value={$outputDeviceId}>
      <option value={null}>— select —</option>
      {#each outputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>
  <button on:click={refresh}>Refresh</button>
  {#if loadError}<span class="err">{loadError}</span>{/if}
</div>

<style>
  .bar {
    display: flex;
    gap: 1rem;
    align-items: center;
  }
  label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    font-size: 12px;
    color: var(--text-1);
  }
  select {
    min-width: 200px;
  }
  .err {
    color: var(--danger);
    font-size: 12px;
  }
</style>
```

---

### Task 1.10: MeterBar component

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\MeterBar.svelte`

- [ ] **Step 1: Implement meter rendering**

Create `src/lib/components/MeterBar.svelte`:

```svelte
<script lang="ts">
  export let label: string;
  export let peakDb: number = -90;
  export let rmsDb: number = -90;

  $: peakPct = dbToPct(peakDb);
  $: rmsPct = dbToPct(rmsDb);
  $: clipping = peakDb > -0.5;

  function dbToPct(db: number): number {
    // Map -60 dB ... 0 dB to 0 ... 100 %.
    const clamped = Math.max(-60, Math.min(0, db));
    return ((clamped + 60) / 60) * 100;
  }
</script>

<div class="meter">
  <span class="label">{label}</span>
  <div class="track">
    <div class="rms" style="width: {rmsPct}%"></div>
    <div class="peak" class:clipping style="left: {peakPct}%"></div>
  </div>
  <span class="value">{peakDb.toFixed(1)} dB</span>
</div>

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 11px;
    min-width: 220px;
  }
  .label {
    color: var(--text-1);
    width: 44px;
  }
  .track {
    position: relative;
    flex: 1;
    height: 10px;
    background: var(--bg-2);
    border-radius: 3px;
    overflow: hidden;
  }
  .rms {
    height: 100%;
    background: linear-gradient(90deg, var(--ok), var(--warn), var(--danger));
    transition: width 30ms linear;
  }
  .peak {
    position: absolute;
    top: 0;
    width: 2px;
    height: 100%;
    background: var(--text-0);
    transition: left 30ms linear;
  }
  .peak.clipping {
    background: var(--danger);
    box-shadow: 0 0 4px var(--danger);
  }
  .value {
    color: var(--text-2);
    width: 50px;
    text-align: right;
  }
</style>
```

---

### Task 1.11: Wire Start/Stop + meter events in App.svelte

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte`

- [ ] **Step 1: Replace App.svelte with wired version**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import DeviceBar from "./lib/components/DeviceBar.svelte";
  import MeterBar from "./lib/components/MeterBar.svelte";
  import { ipc, events } from "./lib/ipc";
  import {
    settings,
    inputDeviceId,
    outputDeviceId,
    engineRunning,
    meters,
    engineError,
  } from "./lib/stores";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let unsubs: UnlistenFn[] = [];

  onMount(async () => {
    const s = await ipc.loadSettings();
    settings.set(s);
    if (s.input_device_id) inputDeviceId.set(s.input_device_id);
    if (s.output_device_id) outputDeviceId.set(s.output_device_id);

    engineRunning.set(await ipc.engineRunning());

    unsubs.push(await events.onMeters((m) => meters.set(m)));
    unsubs.push(
      await events.onEngineState((s) => {
        engineRunning.set(s.running);
        engineError.set(s.error);
      })
    );
  });

  onDestroy(() => {
    unsubs.forEach((u) => u());
  });

  async function toggleEngine() {
    engineError.set(null);
    try {
      if ($engineRunning) {
        await ipc.stopEngine();
      } else {
        if (!$inputDeviceId || !$outputDeviceId) {
          engineError.set("pick input and output devices first");
          return;
        }
        await ipc.startEngine($inputDeviceId, $outputDeviceId);
        // Persist selection.
        const s = await ipc.loadSettings();
        s.input_device_id = $inputDeviceId;
        s.output_device_id = $outputDeviceId;
        await ipc.saveSettings(s);
      }
    } catch (e) {
      engineError.set(String(e));
    }
  }
</script>

<div class="shell">
  <header class="top-bar">
    <strong>RatMic</strong>
    <DeviceBar />
  </header>

  <main class="body">
    <aside class="sidebar"><h3>Presets</h3><p class="muted">(coming soon)</p></aside>
    <section class="chain"><h3>Effect Chain</h3><p class="muted">(coming soon)</p></section>
    <aside class="params"><h3>Parameters</h3><p class="muted">(coming soon)</p></aside>
  </main>

  <footer class="bottom-bar">
    <MeterBar label="In" peakDb={$meters.input_peak_db} rmsDb={$meters.input_rms_db} />
    <MeterBar label="Out" peakDb={$meters.output_peak_db} rmsDb={$meters.output_rms_db} />
    <div class="spacer"></div>
    {#if $engineError}<span class="err">{$engineError}</span>{/if}
    <button class:primary={!$engineRunning} on:click={toggleEngine}>
      {$engineRunning ? "STOP" : "START"}
    </button>
  </footer>
</div>

<style>
  .shell { display: grid; grid-template-rows: 44px 1fr 64px; height: 100%; }
  .top-bar {
    display: flex; align-items: center; gap: 1rem; padding: 0 0.75rem;
    background: var(--bg-1); border-bottom: 1px solid var(--border);
  }
  .body {
    display: grid; grid-template-columns: 180px 1fr 280px; min-height: 0;
  }
  .sidebar, .chain, .params { padding: 0.75rem; overflow: auto; }
  .sidebar { background: var(--bg-1); border-right: 1px solid var(--border); }
  .params { background: var(--bg-1); border-left: 1px solid var(--border); }
  .bottom-bar {
    display: flex; align-items: center; gap: 1rem; padding: 0 0.75rem;
    background: var(--bg-1); border-top: 1px solid var(--border);
  }
  .spacer { flex: 1; }
  .muted { color: var(--text-2); font-size: 12px; }
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .err { color: var(--danger); font-size: 12px; }
</style>
```

---

### Task 1.12: Phase 1 smoke test

- [ ] **Step 1: Launch the app**

Run: `npm run tauri dev`
Expected: window opens, top bar shows DeviceBar with populated dropdowns.

- [ ] **Step 2: Pick devices and start**

- Select your real microphone as Input.
- Select a non-default output (use VB-CABLE Input if available; otherwise temporarily pick speakers — be aware you'll hear yourself).
- Click START.

Expected:
- Button changes to STOP.
- Input meter moves when you speak.
- Output meter moves matching.
- Speaking should be audible on the chosen output (or arrive in Discord if routed through VB-CABLE).

- [ ] **Step 3: Verify settings persistence**

Stop the app, relaunch. The previously-selected input/output device should pre-populate.

- [ ] **Step 4: Verify Rust tests still pass**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 5: Estimate latency manually**

If you have a phone, play a click sound near the mic while monitoring on speakers/headphones. Difference between original and processed click is approximate RTT. Target ≤ 30 ms (a single click should sound "doubled" with minimal echo). If you hear obvious echo, log the input device SR vs internal SR — Phase 1 does not resample, so SR mismatch is the most likely cause. Document the latency observation in a note for review.

---

## Phase 2 — Effect Chain Framework

### Task 2.1: Effect trait

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Add module to main.rs**

Edit `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod effects;
mod events;
mod settings;

use commands::AppState;

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::load_settings,
            commands::save_settings,
            commands::list_audio_devices,
            commands::start_engine,
            commands::stop_engine,
            commands::engine_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Define Effect trait**

Create `src-tauri/src/effects/mod.rs`:

```rust
//! Effect trait and module roots.

pub mod chain;
pub mod crossfade;
pub mod gain;
pub mod limiter;

use anyhow::Result;
use serde_json::Value as Json;

pub trait Effect: Send {
    /// Unique kind identifier, e.g. "gain", "limiter".
    fn type_name(&self) -> &'static str;
    /// Process a mono buffer in place.
    fn process(&mut self, buffer: &mut [f32]);
    /// Replace internal parameters from JSON. Clamps invalid values.
    fn set_params(&mut self, params: &Json) -> Result<()>;
    /// Serialize current parameters.
    fn get_params(&self) -> Json;
    /// Reset internal state (history, filters, RNG).
    fn reset(&mut self);
}
```

- [ ] **Step 3: Verify it compiles (will fail until 2.2–2.5 land)**

The plan order is: trait → crossfade → chain → gain → limiter. After 2.2 the crate compiles again. For now, expect compile errors referencing the missing submodules.

Run: `cd src-tauri && cargo build`
Expected: errors complaining about missing `chain`, `crossfade`, `gain`, `limiter` modules. That's correct — they land next.

---

### Task 2.2: Bypass crossfade utility (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\crossfade.rs`

- [ ] **Step 1: Write tests + implementation**

Create `src-tauri/src/effects/crossfade.rs`:

```rust
//! Linear bypass crossfade.
//!
//! When an effect's `enabled` flag changes, the chain processes the effect
//! over a `mix` that ramps 0→1 (enabling) or 1→0 (disabling) across
//! `RAMP_SAMPLES` to avoid clicks.

const RAMP_MS: f32 = 5.0;

pub struct BypassRamp {
    /// Current mix factor; 0.0 = full dry, 1.0 = full wet (processed).
    pub mix: f32,
    /// Target mix factor.
    pub target: f32,
    /// Per-sample increment toward target.
    pub step: f32,
}

impl BypassRamp {
    pub fn new(sample_rate: u32, initial_enabled: bool) -> Self {
        let ramp_samples = (sample_rate as f32) * (RAMP_MS * 0.001);
        let step = 1.0 / ramp_samples.max(1.0);
        let mix = if initial_enabled { 1.0 } else { 0.0 };
        Self { mix, target: mix, step }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.target = if enabled { 1.0 } else { 0.0 };
    }

    /// Advance ramp by one sample and return current mix factor.
    pub fn tick(&mut self) -> f32 {
        if (self.target - self.mix).abs() < f32::EPSILON {
            return self.mix;
        }
        if self.target > self.mix {
            self.mix = (self.mix + self.step).min(self.target);
        } else {
            self.mix = (self.mix - self.step).max(self.target);
        }
        self.mix
    }

    pub fn is_at_target(&self) -> bool {
        (self.target - self.mix).abs() < f32::EPSILON
    }

    pub fn is_fully_bypassed(&self) -> bool {
        self.mix <= f32::EPSILON && self.target <= f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_default_starts_full_wet() {
        let r = BypassRamp::new(48000, true);
        assert_eq!(r.mix, 1.0);
    }

    #[test]
    fn disable_ramps_down_in_about_5ms() {
        let mut r = BypassRamp::new(48000, true);
        r.set_enabled(false);
        let ramp_samples = (48000.0 * 0.005) as usize;
        for _ in 0..ramp_samples + 1 {
            r.tick();
        }
        assert!(r.is_at_target());
        assert_eq!(r.mix, 0.0);
    }

    #[test]
    fn enable_ramps_up_in_about_5ms() {
        let mut r = BypassRamp::new(48000, false);
        r.set_enabled(true);
        let ramp_samples = (48000.0 * 0.005) as usize;
        for _ in 0..ramp_samples + 1 {
            r.tick();
        }
        assert!(r.is_at_target());
        assert_eq!(r.mix, 1.0);
    }

    #[test]
    fn ticking_at_target_is_idempotent() {
        let mut r = BypassRamp::new(48000, true);
        let before = r.mix;
        for _ in 0..1000 {
            r.tick();
        }
        assert_eq!(r.mix, before);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test effects::crossfade`
Expected: 4 tests pass. (You may need to land 2.3, 2.4, 2.5 first if the crate refuses to build due to missing modules. If so, add empty `pub fn placeholder() {}` shims in `chain.rs`, `gain.rs`, `limiter.rs` to satisfy the module tree, then run again.)

---

### Task 2.3: Effect chain container (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\chain.rs`

- [ ] **Step 1: Implement chain**

Create `src-tauri/src/effects/chain.rs`:

```rust
//! Ordered effect chain with per-effect bypass crossfade.
//!
//! The chain processes each effect into a wet buffer, then blends wet/dry
//! per sample using the effect's BypassRamp.

use super::crossfade::BypassRamp;
use super::Effect;

pub struct EffectSlot {
    pub effect: Box<dyn Effect>,
    pub enabled: bool,
    ramp: BypassRamp,
    /// Scratch buffer for wet samples.
    wet: Vec<f32>,
}

impl EffectSlot {
    pub fn new(effect: Box<dyn Effect>, enabled: bool, sample_rate: u32) -> Self {
        Self {
            effect,
            enabled,
            ramp: BypassRamp::new(sample_rate, enabled),
            wet: Vec::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.ramp.set_enabled(enabled);
    }

    /// Process `buffer` through this slot in place, blending wet/dry by ramp.
    pub fn process(&mut self, buffer: &mut [f32]) {
        if self.ramp.is_fully_bypassed() {
            return;
        }
        self.wet.clear();
        self.wet.extend_from_slice(buffer);
        self.effect.process(&mut self.wet);
        for i in 0..buffer.len() {
            let mix = self.ramp.tick();
            buffer[i] = buffer[i] * (1.0 - mix) + self.wet[i] * mix;
        }
    }
}

pub struct EffectChain {
    slots: Vec<EffectSlot>,
    sample_rate: u32,
}

impl EffectChain {
    pub fn new(sample_rate: u32) -> Self {
        Self { slots: Vec::new(), sample_rate }
    }

    pub fn push(&mut self, effect: Box<dyn Effect>, enabled: bool) {
        self.slots
            .push(EffectSlot::new(effect, enabled, self.sample_rate));
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(slot) = self.slots.get_mut(index) {
            slot.set_enabled(enabled);
        }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for slot in &mut self.slots {
            slot.process(buffer);
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde_json::Value as Json;

    /// Test effect that multiplies samples by a constant.
    struct Mul(f32);
    impl Effect for Mul {
        fn type_name(&self) -> &'static str { "mul" }
        fn process(&mut self, buffer: &mut [f32]) {
            for s in buffer { *s *= self.0; }
        }
        fn set_params(&mut self, _: &Json) -> Result<()> { Ok(()) }
        fn get_params(&self) -> Json { Json::Null }
        fn reset(&mut self) {}
    }

    #[test]
    fn empty_chain_passes_through() {
        let mut c = EffectChain::new(48000);
        let mut buf = vec![0.5; 64];
        c.process(&mut buf);
        for s in &buf { assert_eq!(*s, 0.5); }
    }

    #[test]
    fn enabled_effect_processes_after_ramp() {
        let mut c = EffectChain::new(48000);
        c.push(Box::new(Mul(2.0)), true);
        // Run enough samples to fully ramp up (5 ms @ 48 kHz = 240 samples).
        let mut warm = vec![0.5; 500];
        c.process(&mut warm);
        // Tail samples after ramp should be 0.5 * 2.0 = 1.0.
        for s in &warm[260..] {
            assert!((*s - 1.0).abs() < 1e-3, "post-ramp expected ~1.0, got {}", s);
        }
    }

    #[test]
    fn disabled_effect_is_dry() {
        let mut c = EffectChain::new(48000);
        c.push(Box::new(Mul(2.0)), false);
        let mut buf = vec![0.5; 500];
        c.process(&mut buf);
        for s in &buf { assert_eq!(*s, 0.5); }
    }

    #[test]
    fn toggle_does_not_produce_discontinuity() {
        let mut c = EffectChain::new(48000);
        c.push(Box::new(Mul(2.0)), true);
        let mut buf = vec![0.5; 500];
        c.process(&mut buf);
        c.set_enabled(0, false);
        let mut buf2 = vec![0.5; 500];
        c.process(&mut buf2);
        // Across the ramp, neighbouring samples must not jump by more than ~step*max_signal.
        // 0.5 signal, 240-sample ramp → max ≈ 0.5/240 ≈ 0.0021. Allow 0.05 slack.
        for w in buf2.windows(2) {
            let diff = (w[1] - w[0]).abs();
            assert!(diff < 0.05, "discontinuity {} between samples", diff);
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test effects::chain`
Expected: 4 tests pass.

---

### Task 2.4: Gain effect (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\gain.rs`

- [ ] **Step 1: Implement Gain effect with TDD**

Create `src-tauri/src/effects/gain.rs`:

```rust
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
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test effects::gain`
Expected: 4 tests pass.

---

### Task 2.5: Final limiter (TDD)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\limiter.rs`

- [ ] **Step 1: Implement peak limiter with TDD**

Create `src-tauri/src/effects/limiter.rs`:

```rust
//! Peak limiter with smooth attack/release.
//!
//! No lookahead: a single-sample peak detector with exponential envelope.
//! Sufficient for ear-safety; for "brick-wall" guarantees we'd need lookahead.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterParams {
    #[serde(rename = "ceilingDb", default = "default_ceiling")]
    pub ceiling_db: f32,
    #[serde(rename = "releaseMs", default = "default_release")]
    pub release_ms: f32,
}

fn default_ceiling() -> f32 { -3.0 }
fn default_release() -> f32 { 80.0 }

impl Default for LimiterParams {
    fn default() -> Self {
        Self { ceiling_db: default_ceiling(), release_ms: default_release() }
    }
}

const MIN_CEILING_DB: f32 = -24.0;
const MAX_CEILING_DB: f32 = 0.0;
const MIN_RELEASE_MS: f32 = 1.0;
const MAX_RELEASE_MS: f32 = 500.0;
const ATTACK_MS: f32 = 5.0;

pub struct Limiter {
    params: LimiterParams,
    sample_rate: u32,
    ceiling_amp: f32,
    gain: f32,
    attack_coef: f32,
    release_coef: f32,
    /// True if any sample in the most recent process() call required limiting.
    pub was_active: bool,
}

impl Limiter {
    pub fn new(sample_rate: u32, params: LimiterParams) -> Self {
        let mut l = Self {
            params,
            sample_rate,
            ceiling_amp: 1.0,
            gain: 1.0,
            attack_coef: 0.0,
            release_coef: 0.0,
            was_active: false,
        };
        l.recompute();
        l
    }

    fn recompute(&mut self) {
        let sr = self.sample_rate as f32;
        self.ceiling_amp = 10.0_f32.powf(self.params.ceiling_db / 20.0);
        self.attack_coef = (-1.0 / (ATTACK_MS * 0.001 * sr)).exp();
        self.release_coef = (-1.0 / (self.params.release_ms * 0.001 * sr)).exp();
    }
}

impl Effect for Limiter {
    fn type_name(&self) -> &'static str { "limiter" }

    fn process(&mut self, buffer: &mut [f32]) {
        let ceiling = self.ceiling_amp;
        let attack = self.attack_coef;
        let release = self.release_coef;
        let mut gain = self.gain;
        let mut active = false;
        for s in buffer {
            let abs = s.abs();
            let target_gain = if abs * gain > ceiling {
                active = true;
                ceiling / abs
            } else {
                1.0
            };
            let coef = if target_gain < gain { attack } else { release };
            gain = target_gain + (gain - target_gain) * coef;
            *s *= gain;
            if s.abs() > ceiling {
                *s = s.signum() * ceiling;
            }
        }
        self.gain = gain;
        self.was_active = active;
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: LimiterParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.ceiling_db = p.ceiling_db.clamp(MIN_CEILING_DB, MAX_CEILING_DB);
        p.release_ms = p.release_ms.clamp(MIN_RELEASE_MS, MAX_RELEASE_MS);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("limiter params serialize")
    }

    fn reset(&mut self) {
        self.gain = 1.0;
        self.was_active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_signal_passes_through_unchanged() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        let mut buf = vec![0.1; 256];
        let original = buf.clone();
        l.process(&mut buf);
        for (a, b) in buf.iter().zip(original.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
        assert!(!l.was_active);
    }

    #[test]
    fn loud_signal_is_clamped_to_ceiling() {
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        let ceiling_amp = 10.0_f32.powf(-3.0 / 20.0);
        let mut buf = vec![0.95; 4800]; // 100 ms of loud signal
        l.process(&mut buf);
        // After attack settles, samples should be at or below ceiling.
        for (i, s) in buf.iter().enumerate().skip(1000) {
            assert!(
                s.abs() <= ceiling_amp + 1e-3,
                "sample {} = {} exceeds ceiling {}",
                i, s, ceiling_amp
            );
        }
        assert!(l.was_active);
    }

    #[test]
    fn negative_peaks_also_clamped() {
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        let ceiling_amp = 10.0_f32.powf(-3.0 / 20.0);
        let mut buf = vec![-0.95; 4800];
        l.process(&mut buf);
        for s in buf.iter().skip(1000) {
            assert!(s.abs() <= ceiling_amp + 1e-3);
        }
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        l.set_params(&serde_json::json!({ "ceilingDb": 10.0, "releaseMs": 99999.0 })).unwrap();
        let p: LimiterParams = serde_json::from_value(l.get_params()).unwrap();
        assert_eq!(p.ceiling_db, MAX_CEILING_DB);
        assert_eq!(p.release_ms, MAX_RELEASE_MS);
    }

    #[test]
    fn reset_clears_gain_state() {
        let mut l = Limiter::new(48000, LimiterParams::default());
        let mut buf = vec![0.95; 1000];
        l.process(&mut buf);
        assert!(l.gain < 1.0);
        l.reset();
        assert_eq!(l.gain, 1.0);
        assert!(!l.was_active);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test effects::limiter`
Expected: 5 tests pass.

- [ ] **Step 3: Run all Rust tests to make sure nothing else broke**

Run: `cd src-tauri && cargo test`
Expected: all tests across `settings`, `audio::*`, `effects::*` pass.

---

### Task 2.6: Wire effect chain into audio engine

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\engine.rs`

- [ ] **Step 1: Add chain ownership to the engine**

Replace `src-tauri/src/audio/engine.rs`:

```rust
//! Top-level audio engine: owns input stream, worker thread, output backend, effect chain.

use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::effects::chain::EffectChain;
use crate::effects::gain::{Gain, GainParams};
use crate::effects::limiter::{Limiter, LimiterParams};

use super::devices::{find_input_device, find_output_device};
use super::format::AudioFormat;
use super::input_stream::InputStream;
use super::meters::{Meter, MeterValue};
use super::output_backend::AudioOutputBackend;
use super::ring_buffer::{AudioRing, RingConsumer};
use super::system_output::SystemDeviceBackend;
// Note: RingProducer/InputStream are moved by value (no Arc<Mutex<>>) — see Task 1.6 fix.

pub const INTERNAL_SAMPLE_RATE: u32 = 48_000;
const INPUT_RING_CAPACITY: usize = 8192;
const WORKER_CHUNK_SAMPLES: usize = 480;
const METER_TICK_MS: u64 = 16;

#[derive(Debug, Clone, Copy)]
pub struct MeterSnapshot {
    pub input: MeterValue,
    pub output: MeterValue,
}

pub trait MeterSink: Send + 'static {
    fn push(&self, snap: MeterSnapshot);
}

pub struct AudioEngine {
    _input: InputStream,
    worker_handle: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    pub chain: Arc<Mutex<EffectChain>>,
}

impl AudioEngine {
    pub fn start<S: MeterSink + 'static>(
        input_id: &str,
        output_id: &str,
        meter_sink: S,
    ) -> Result<Self> {
        if input_id == output_id {
            return Err(anyhow!("input and output device must differ"));
        }
        let input_device = find_input_device(input_id)
            .with_context(|| format!("opening input device {input_id}"))?;
        let output_device = find_output_device(output_id)
            .with_context(|| format!("opening output device {output_id}"))?;

        let (in_prod, in_cons) = AudioRing::new(INPUT_RING_CAPACITY);
        let input_stream = InputStream::open(&input_device, in_prod)
            .context("opening input stream")?;

        let mut backend = SystemDeviceBackend::new(output_device);
        backend
            .open(AudioFormat {
                sample_rate: INTERNAL_SAMPLE_RATE,
                channels: 1,
            })
            .context("opening output backend")?;
        let backend: Arc<Mutex<Box<dyn AudioOutputBackend>>> =
            Arc::new(Mutex::new(Box::new(backend)));

        // Build default chain: Gain (disabled) → Limiter (always enabled).
        let mut chain = EffectChain::new(INTERNAL_SAMPLE_RATE);
        chain.push(
            Box::new(Gain::new(GainParams::default())),
            false,
        );
        chain.push(
            Box::new(Limiter::new(INTERNAL_SAMPLE_RATE, LimiterParams::default())),
            true,
        );
        let chain = Arc::new(Mutex::new(chain));

        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            let backend = backend.clone();
            let chain = chain.clone();
            thread::Builder::new()
                .name("ratmic-audio-worker".into())
                .spawn(move || {
                    worker_loop(in_cons, backend, chain, meter_sink, stop);
                })
                .context("spawning audio worker")?
        };

        log::info!("audio engine started with chain ({} slots)", chain.lock().len());

        Ok(Self {
            _input: input_stream,
            worker_handle: Some(worker),
            stop_flag: stop,
            backend,
            chain,
        })
    }

    pub fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        self.backend.lock().close();
        log::info!("audio engine stopped");
    }
}

fn worker_loop<S: MeterSink>(
    mut consumer: RingConsumer,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    chain: Arc<Mutex<EffectChain>>,
    sink: S,
    stop: Arc<AtomicBool>,
) {
    let mut buffer = vec![0.0_f32; WORKER_CHUNK_SAMPLES];
    let mut in_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let mut out_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let meter_interval = Duration::from_millis(METER_TICK_MS);
    let mut last_meter = std::time::Instant::now();

    while !stop.load(Ordering::Relaxed) {
        let n = consumer.pop(&mut buffer);
        if n == 0 {
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        let chunk = &mut buffer[..n];
        in_meter.process(chunk);
        chain.lock().process(chunk);
        out_meter.process(chunk);
        let _ = backend.lock().write(chunk);

        if last_meter.elapsed() >= meter_interval {
            sink.push(MeterSnapshot {
                input: in_meter.snapshot(),
                output: out_meter.snapshot(),
            });
            last_meter = std::time::Instant::now();
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build`
Expected: compiles cleanly.

---

### Task 2.7: Effect chain IPC commands

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\commands.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\main.rs`

- [ ] **Step 1: Replace `src-tauri/src/commands.rs` entirely with this**

This is a full file replacement (not an append). The imports near the top are the same as Task 1.8 plus two new lines for `GainParams` and `Value as Json`. The existing settings/audio command bodies are preserved verbatim.

```rust
use parking_lot::Mutex;
use serde_json::Value as Json;
use tauri::{AppHandle, Emitter, State};

use crate::audio::devices::{list_devices, DeviceInfo};
use crate::audio::engine::{AudioEngine, MeterSink, MeterSnapshot};
use crate::effects::gain::GainParams;
use crate::events::{EngineStateEvent, MeterEvent, EVENT_ENGINE_STATE, EVENT_METERS};
use crate::settings::Settings;

pub struct AppState {
    pub engine: Mutex<Option<AudioEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
        }
    }
}

struct EmitSink {
    app: AppHandle,
}

impl MeterSink for EmitSink {
    fn push(&self, snap: MeterSnapshot) {
        let ev = MeterEvent {
            input_peak_db: snap.input.peak_db(),
            input_rms_db: snap.input.rms_db(),
            output_peak_db: snap.output.peak_db(),
            output_rms_db: snap.output.rms_db(),
        };
        let _ = self.app.emit(EVENT_METERS, ev);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChainSlotView {
    pub index: usize,
    pub type_name: String,
    pub enabled: bool,
    pub params: Json,
}

#[tauri::command]
pub fn load_settings() -> Result<Settings, String> {
    Settings::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<DeviceInfo>, String> {
    list_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_engine(
    app: AppHandle,
    state: State<'_, AppState>,
    input_id: String,
    output_id: String,
) -> Result<(), String> {
    let mut guard = state.engine.lock();
    if guard.is_some() {
        return Err("engine already running".into());
    }
    let sink = EmitSink { app: app.clone() };
    let engine = AudioEngine::start(&input_id, &output_id, sink)
        .map_err(|e| e.to_string())?;
    *guard = Some(engine);
    let _ = app.emit(
        EVENT_ENGINE_STATE,
        EngineStateEvent {
            running: true,
            error: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn stop_engine(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.engine.lock();
    if let Some(engine) = guard.take() {
        engine.stop();
    }
    let _ = app.emit(
        EVENT_ENGINE_STATE,
        EngineStateEvent {
            running: false,
            error: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub fn engine_running(state: State<'_, AppState>) -> bool {
    state.engine.lock().is_some()
}

#[tauri::command]
pub fn get_chain(state: State<'_, AppState>) -> Vec<ChainSlotView> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        // Engine not running → return a default view so the UI has something to render.
        return vec![
            ChainSlotView {
                index: 0,
                type_name: "gain".into(),
                enabled: false,
                params: serde_json::to_value(GainParams::default()).unwrap(),
            },
            ChainSlotView {
                index: 1,
                type_name: "limiter".into(),
                enabled: true,
                params: serde_json::json!({ "ceilingDb": -3.0, "releaseMs": 80.0 }),
            },
        ];
    };
    let chain = engine.chain.lock();
    chain
        .slots_view()
        .into_iter()
        .enumerate()
        .map(|(i, (type_name, enabled, params))| ChainSlotView {
            index: i,
            type_name: type_name.into(),
            enabled,
            params,
        })
        .collect()
}

#[tauri::command]
pub fn set_effect_enabled(
    state: State<'_, AppState>,
    index: usize,
    enabled: bool,
) -> Result<(), String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    engine.chain.lock().set_enabled(index, enabled);
    Ok(())
}

#[tauri::command]
pub fn set_effect_params(
    state: State<'_, AppState>,
    index: usize,
    params: Json,
) -> Result<(), String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    engine
        .chain
        .lock()
        .set_params(index, &params)
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Add `slots_view()` and `set_params()` helpers on EffectChain**

Edit `src-tauri/src/effects/chain.rs` and add inside `impl EffectChain`:

```rust
pub fn slots_view(&self) -> Vec<(&'static str, bool, serde_json::Value)> {
    self.slots
        .iter()
        .map(|s| (s.effect.type_name(), s.enabled, s.effect.get_params()))
        .collect()
}

pub fn set_params(&mut self, index: usize, params: &serde_json::Value) -> anyhow::Result<()> {
    let Some(slot) = self.slots.get_mut(index) else {
        return Err(anyhow::anyhow!("slot index {} out of range", index));
    };
    slot.effect.set_params(params)
}
```

- [ ] **Step 3: Register commands in main.rs**

Update the `invoke_handler!` macro in `src-tauri/src/main.rs`:

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
])
```

- [ ] **Step 4: Verify it compiles and tests still pass**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

---

### Task 2.8: EffectChain UI component

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\EffectChain.svelte`
- Modify: `E:\ClaudeCode\ratmic\src\lib\ipc.ts`

- [ ] **Step 1: Extend ipc.ts**

Append to `src/lib/ipc.ts` (above the `events` object):

```ts
export interface ChainSlotView {
  index: number;
  type_name: string;
  enabled: boolean;
  params: unknown;
}

// Extend the existing `ipc` const declaration:
//   getChain, setEffectEnabled, setEffectParams
```

Then replace the `ipc` const block to include the new methods:

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
};
```

- [ ] **Step 2: Add `selectedEffect` store**

Append to `src/lib/stores.ts`:

```ts
import type { ChainSlotView } from "./ipc";

export const chain = writable<ChainSlotView[]>([]);
export const selectedEffectIndex = writable<number | null>(null);
```

- [ ] **Step 3: EffectChain component**

Create `src/lib/components/EffectChain.svelte`:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../ipc";
  import { chain, selectedEffectIndex, engineRunning } from "../stores";

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
      {#if slot.type_name === "limiter"}<span class="badge">fixed</span>{/if}
    </li>
  {/each}
</ul>
{#if !$engineRunning}
  <p class="muted">Start the engine to toggle effects.</p>
{/if}

<style>
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
  li:hover {
    background: var(--bg-3);
  }
  li.selected {
    border-color: var(--accent);
  }
  li.fixed {
    opacity: 0.85;
  }
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
  .muted {
    color: var(--text-2);
    font-size: 12px;
    margin-top: 0.5rem;
  }
</style>
```

---

### Task 2.9: EffectParams component

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\EffectParams.svelte`
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte`

- [ ] **Step 1: Implement params panel**

Create `src/lib/components/EffectParams.svelte`:

```svelte
<script lang="ts">
  import { ipc } from "../ipc";
  import { chain, selectedEffectIndex } from "../stores";

  $: slot = $selectedEffectIndex !== null
    ? $chain.find((s) => s.index === $selectedEffectIndex)
    : null;

  async function setGainDb(value: number) {
    if (!slot) return;
    try {
      await ipc.setEffectParams(slot.index, { gainDb: value });
      chain.update((items) =>
        items.map((s) =>
          s.index === slot.index ? { ...s, params: { gainDb: value } } : s
        )
      );
    } catch (e) {
      console.error(e);
    }
  }

  async function setLimiterCeiling(value: number) {
    if (!slot) return;
    const params = { ...(slot.params as object), ceilingDb: value };
    try {
      await ipc.setEffectParams(slot.index, params);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<h3>Parameters</h3>

{#if !slot}
  <p class="muted">Select an effect to edit its parameters.</p>
{:else if slot.type_name === "gain"}
  {@const params = slot.params as { gainDb?: number }}
  <label>
    Gain
    <input
      type="range"
      min="-24"
      max="24"
      step="0.5"
      value={params.gainDb ?? 0}
      on:input={(e) => setGainDb(parseFloat((e.target as HTMLInputElement).value))}
    />
    <span class="value">{(params.gainDb ?? 0).toFixed(1)} dB</span>
  </label>
{:else if slot.type_name === "limiter"}
  {@const params = slot.params as { ceilingDb?: number; releaseMs?: number }}
  <label>
    Ceiling
    <input
      type="range"
      min="-24"
      max="0"
      step="0.5"
      value={params.ceilingDb ?? -3}
      on:input={(e) =>
        setLimiterCeiling(parseFloat((e.target as HTMLInputElement).value))}
    />
    <span class="value">{(params.ceilingDb ?? -3).toFixed(1)} dB</span>
  </label>
  <p class="muted">Release: {params.releaseMs ?? 80} ms (fixed for now)</p>
{:else}
  <p class="muted">No editor for "{slot.type_name}" yet.</p>
{/if}

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 12px;
    color: var(--text-1);
    margin-bottom: 0.75rem;
  }
  input[type="range"] {
    width: 100%;
  }
  .value {
    color: var(--text-0);
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--text-2);
    font-size: 12px;
  }
</style>
```

- [ ] **Step 2: Mount components in App.svelte**

Replace `src/App.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import DeviceBar from "./lib/components/DeviceBar.svelte";
  import MeterBar from "./lib/components/MeterBar.svelte";
  import EffectChain from "./lib/components/EffectChain.svelte";
  import EffectParams from "./lib/components/EffectParams.svelte";
  import { ipc, events } from "./lib/ipc";
  import {
    settings,
    inputDeviceId,
    outputDeviceId,
    engineRunning,
    meters,
    engineError,
  } from "./lib/stores";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let unsubs: UnlistenFn[] = [];

  onMount(async () => {
    const s = await ipc.loadSettings();
    settings.set(s);
    if (s.input_device_id) inputDeviceId.set(s.input_device_id);
    if (s.output_device_id) outputDeviceId.set(s.output_device_id);

    engineRunning.set(await ipc.engineRunning());

    unsubs.push(await events.onMeters((m) => meters.set(m)));
    unsubs.push(
      await events.onEngineState((s) => {
        engineRunning.set(s.running);
        engineError.set(s.error);
      })
    );
  });

  onDestroy(() => unsubs.forEach((u) => u()));

  async function toggleEngine() {
    engineError.set(null);
    try {
      if ($engineRunning) {
        await ipc.stopEngine();
      } else {
        if (!$inputDeviceId || !$outputDeviceId) {
          engineError.set("pick input and output devices first");
          return;
        }
        await ipc.startEngine($inputDeviceId, $outputDeviceId);
        const s = await ipc.loadSettings();
        s.input_device_id = $inputDeviceId;
        s.output_device_id = $outputDeviceId;
        await ipc.saveSettings(s);
      }
    } catch (e) {
      engineError.set(String(e));
    }
  }
</script>

<div class="shell">
  <header class="top-bar">
    <strong>RatMic</strong>
    <DeviceBar />
  </header>

  <main class="body">
    <aside class="sidebar"><h3>Presets</h3><p class="muted">(coming soon)</p></aside>
    <section class="chain"><EffectChain /></section>
    <aside class="params"><EffectParams /></aside>
  </main>

  <footer class="bottom-bar">
    <MeterBar label="In" peakDb={$meters.input_peak_db} rmsDb={$meters.input_rms_db} />
    <MeterBar label="Out" peakDb={$meters.output_peak_db} rmsDb={$meters.output_rms_db} />
    <div class="spacer"></div>
    {#if $engineError}<span class="err">{$engineError}</span>{/if}
    <button class:primary={!$engineRunning} on:click={toggleEngine}>
      {$engineRunning ? "STOP" : "START"}
    </button>
  </footer>
</div>

<style>
  .shell { display: grid; grid-template-rows: 44px 1fr 64px; height: 100%; }
  .top-bar { display: flex; align-items: center; gap: 1rem; padding: 0 0.75rem; background: var(--bg-1); border-bottom: 1px solid var(--border); }
  .body { display: grid; grid-template-columns: 180px 1fr 280px; min-height: 0; }
  .sidebar, .chain, .params { padding: 0.75rem; overflow: auto; }
  .sidebar { background: var(--bg-1); border-right: 1px solid var(--border); }
  .params { background: var(--bg-1); border-left: 1px solid var(--border); }
  .bottom-bar { display: flex; align-items: center; gap: 1rem; padding: 0 0.75rem; background: var(--bg-1); border-top: 1px solid var(--border); }
  .spacer { flex: 1; }
  .muted { color: var(--text-2); font-size: 12px; }
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .err { color: var(--danger); font-size: 12px; }
</style>
```

---

### Task 2.10: Phase 2 smoke test

- [ ] **Step 1: Launch the app**

Run: `npm run tauri dev`
Expected:
- Window opens, sidebar shows "Presets (coming soon)", middle shows Effect Chain with "gain" and "limiter" rows.

- [ ] **Step 2: Start engine, toggle Gain**

- Select devices, click START.
- Click the checkbox next to "gain". Voice should still pass through.
- Click on the "gain" row → params panel shows Gain slider.
- Move slider up to ~+12 dB → your voice should become noticeably louder on output.
- Move slider down to −12 dB → quieter.
- Check the checkbox off and on rapidly → output should not produce clicks or pops.

- [ ] **Step 3: Verify limiter does not become disable-able from UI**

- The "limiter" row's checkbox should be disabled (greyed out).
- Selecting limiter shows ceiling slider; default −3 dB. Lower ceiling to −12 dB; speak loudly — output should be clamped harder.

- [ ] **Step 4: Loud-input safety check**

- Set Gain to +20 dB.
- Speak loudly into the mic.
- Output peak meter should not exceed −3 dB (limiter ceiling default).
- Output meter "clipping" indicator (peak > −0.5 dB) should not fire.

- [ ] **Step 5: All Rust tests still pass**

Run: `cd src-tauri && cargo test`
Expected: all tests pass.

- [ ] **Step 6: TypeScript checks pass**

Run: `npm run check`
Expected: 0 errors.

---

## Final Verification

When all Phase 0–2 tasks are checked off:

- [ ] App launches via `npm run tauri dev` without errors.
- [ ] Device dropdowns populated; selection persists across restarts.
- [ ] Speaking into the mic produces audible output on the chosen device.
- [ ] Input + output meters move in response to speech.
- [ ] Effect chain shows Gain (toggleable) and Limiter (fixed).
- [ ] Gain effect changes volume audibly without clicks on toggle.
- [ ] Limiter blocks loud signals from exceeding −3 dB on output.
- [ ] `cd src-tauri && cargo test` → all green.
- [ ] `npm run check` → 0 errors.
- [ ] No memory growth over a 10-minute run with the engine on (manual check via Task Manager).

Phases 3–6 will be planned in separate documents once this foundation is verified working.
