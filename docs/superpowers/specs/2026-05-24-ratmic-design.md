# RatMic — Design Spec

**Date:** 2026-05-24
**Status:** Approved for planning
**Author:** Claude Code (with user)

---

## 1. Overview

RatMic is a Windows desktop app that captures a user's real microphone, runs the
signal through a customizable chain of intentionally degrading effects, and
sends the result to a selected output device (typically VB-CABLE Input). The
user then picks the matching virtual cable output as their microphone in
Discord, Steam, OBS, games, etc.

This is a **bad-mic simulator**, not a voice changer. The goal is funny,
cursed, broken-sounding audio in real time, with safety guardrails that prevent
ear-damaging output.

## 2. Goals

- Real-time audio capture → effects → output with ≤ 30 ms RTT on a typical
  Windows machine (WASAPI shared mode).
- Modular, data-driven effect chain serializable to JSON.
- Stable for multi-hour Discord sessions, low CPU, no leaks.
- Safe by default: final limiter + output ceiling + Safe Output Mode.
- Architecture isolates the audio engine from the output transport so a
  first-party "RatMic Virtual Microphone" driver can be added later without
  rewriting effects, presets, or UI.

## 3. Non-Goals (v1)

- Custom Windows audio driver (kernel or APO).
- AI voice conversion, Discord/game injection, kernel hacks.
- Accounts, cloud preset marketplace, soundboard, social features.
- Mobile, mac, or Linux ports (architecture must not preclude them).
- DAW-style routing graphs, sidechain matrices, MIDI/OSC.

## 4. Stack & Library Choices

| Concern | Choice | Rationale |
|---|---|---|
| App shell | Tauri 2.x | User-directed. Small footprint, native Rust backend. |
| Frontend | Svelte 4 + TypeScript + Vite | Lightweight reactive UI; React is overkill, vanilla TS is more grunt work. |
| Audio I/O | `cpal` 0.15+ | Production-proven Rust crate; uses WASAPI **shared** mode (doesn't kick other apps off the device). |
| Lock-free queues | `ringbuf` | SPSC, no allocations in audio callbacks. |
| Resampling | `rubato` | Handles SR mismatch between input device, internal 48 kHz pipeline, and output device. |
| DSP primitives | Hand-rolled biquad + `dasp` helpers | ~30 LOC per effect. `fundsp`'s graph model fights the modular chain design. |
| Global hotkeys | `global-hotkey` | Sufficient for v1; v2 may need raw-input hook for fullscreen-exclusive games. |
| Serialization | `serde` + `serde_json` | Versioned schemas via explicit `schema_version` field. |
| Settings paths | `directories` | OS-correct: `%APPDATA%\RatMic\`. |
| Test recording | `hound` | Minimal WAV writer. |

**Why not direct `wasapi` bindings?** cpal already wraps WASAPI shared mode.
Going lower-level saves a few ms at the cost of a week of FFI work. Revisit
only if profiling shows real cpal overhead at < 15 ms latency targets.

**Why not egui?** Tauri+Svelte gives a more polished UI for the same effort
and matches the project's "funny but professional" tone. egui remains a clean
fallback if the JS↔Rust boundary becomes painful — the audio engine doesn't
change.

## 5. Architecture

### 5.1 Process & thread layout

```
   UI (Svelte / webview)
        │ Tauri IPC (commands + events)
        ▼
   Tauri Main (Rust) ──── Settings ──── Presets ──── Diagnostics
        │
        │ start/stop, hotkey events, param updates
        ▼
   ┌─────────────────────────────────────────────────────────┐
   │                    Audio Engine                          │
   │                                                          │
   │  cpal Input ──► InRing ──► Worker Thread                │
   │  (callback)               │                              │
   │                           │  - Resample in (rubato)      │
   │                           │  - Effect chain              │
   │                           │  - Final limiter (fixed)     │
   │                           │  - Meters / clip stats       │
   │                           │  - Resample out (rubato)     │
   │                           ▼                              │
   │                       Splitter ──► OutRing ──► cpal Out  │
   │                           │                              │
   │                           ├──► MonitorRing ──► cpal Out  │
   │                           │                              │
   │                           └──► TestRecord buffer         │
   └─────────────────────────────────────────────────────────┘
```

| Thread | Owner | Responsibility |
|---|---|---|
| UI thread | webview | Svelte rendering, user input |
| Tauri main | Rust | Command handlers, settings I/O, hotkey listener, device-watch timer |
| cpal input callback | cpal | Copy raw input samples to `InRing`, nothing else |
| Audio worker | RatMic | Pull `InRing` → resample → effect chain → resample → fan out |
| cpal output callback(s) | cpal | Drain `OutRing` / `MonitorRing` |
| Device-watch timer | RatMic | Poll device list every 2 s, detect disconnects |

### 5.2 Why a worker thread (not DSP-in-callback)

- Decouples I/O timing from processing — transient effect spikes don't
  underrun the output stream.
- Clean fan-out (main output + local monitor + test record) without
  duplicating work in multiple callbacks.
- Easier to instrument and pause/resume independently from device I/O.

### 5.3 Audio format

- Internal pipeline: **48 kHz, f32, mono**. (Mic is mono; if device only
  exposes stereo we downmix.)
- Resampling at the input boundary (device SR → 48 kHz) and output boundary
  (48 kHz → device SR) via `rubato`.
- `cpal::SampleFormat` conversion (i16/u16/f32) handled in `audio::format`.

### 5.4 Buffering & latency budget

- cpal buffer size: request 256–512 samples (~5–10 ms at 48 kHz). Driver may
  override.
- Ring buffers sized to 4× one cpal buffer to absorb jitter.
- Expected RTT (mic → output device): **15–25 ms** typical, ≤ 30 ms target.

## 6. Routing Model & VB-CABLE in v1

VB-CABLE is **just an output device the user picks**. RatMic does not bundle
it, depend on it, or special-case it — it appears as "CABLE Input" in the
output dropdown like any other endpoint.

The routing health check is **advisory only**, never blocking:

| Heuristic | UI signal |
|---|---|
| Output name contains `CABLE` / `Virtual` / `Voicemeeter` / `BlackHole` / `VoiceMod` | ✅ Likely correct |
| Output device == system default playback device | ⚠️ Looks like your speakers — Discord won't hear you |
| Input device == output device | ❌ Same device → feedback loop |
| No output device selected | ❌ Audio engine cannot start |

User can override any warning.

## 7. Future "RatMic Virtual Microphone" Backend

The audio engine writes through a trait:

```rust
trait AudioOutputBackend: Send {
    fn name(&self) -> &str;
    fn open(&mut self, format: AudioFormat) -> Result<()>;
    fn write(&mut self, samples: &[f32]) -> Result<()>;
    fn close(&mut self);
}
```

- v1 ships only `SystemDeviceBackend` (cpal output stream).
- v2 adds `RatMicVirtualMicBackend` that writes into a first-party virtual
  driver (likely APO or WaveRT; requires code signing).
- Everything else — UI, effects, presets, diagnostics — is unchanged when the
  new backend lands. Backend selection becomes a settings option.

## 8. Data Model

### 8.1 Preset schema

```jsonc
{
  "schema_version": 1,
  "name": "Xbox 360 Lobby",
  "description": "Compressed, clipped, noisy old voice chat.",
  "effects": [
    { "type": "gain",       "enabled": true, "params": { "gainDb": 8 } },
    { "type": "bandpass",   "enabled": true, "params": { "lowCutHz": 300, "highCutHz": 3400, "midBoostDb": 4 } },
    { "type": "bitcrusher", "enabled": true, "params": { "bitDepth": 8, "sampleRateHz": 11025, "mix": 0.8 } },
    { "type": "clipper",    "enabled": true, "params": { "drive": 2.5, "hardClip": 0.65, "softClip": 0.3, "outputTrimDb": -4 } },
    { "type": "packetLoss", "enabled": true, "params": { "dropChance": 0.08, "minDropMs": 30, "maxDropMs": 140, "stutterChance": 0.05 } },
    { "type": "limiter",    "enabled": true, "params": { "ceilingDb": -3, "releaseMs": 80 } }
  ]
}
```

**Lenience rules:**
- Unknown `type` → skip with logged warning, don't crash.
- Missing param → fill with effect's default.
- Out-of-range param → clamp to safe range at load.
- Bad JSON → user-visible error; original file untouched.

### 8.2 Settings

```rust
struct Settings {
    schema_version: u32,
    input_device_id: Option<String>,    // persistent name+kind, not handle
    output_device_id: Option<String>,
    monitor_enabled: bool,
    safe_output_mode: bool,             // default true
    hotkey: Option<HotkeyConfig>,
    last_preset_name: Option<String>,
    current_chain: Vec<EffectInstance>, // live chain, separate from saved presets
}

struct HotkeyConfig {
    key: String,                        // e.g. "F8", "Ctrl+Shift+M"
    mode: HotkeyMode,                   // Hold | Toggle
}
```

Stored at `%APPDATA%\RatMic\settings.json`.
User presets at `%APPDATA%\RatMic\presets\*.json`.
Built-in presets bundled via `include_str!` (not on disk).

## 9. Effects (MVP)

Every effect implements:

```rust
trait Effect: Send {
    fn type_id() -> &'static str where Self: Sized;
    fn process(&mut self, buffer: &mut [f32]);
    fn set_params(&mut self, params: &serde_json::Value) -> Result<()>;
    fn get_params(&self) -> serde_json::Value;
    fn reset(&mut self);                // clear internal state, used on chain edits
}
```

The chain holds `Vec<Box<dyn Effect>>`. Each instance also tracks
`enabled: bool` and a short bypass crossfade ramp (~5 ms) on toggle.

### 9.1 Input Gain
- `gainDb`: −24 to +24 dB.
- Pre-gain meter feeds the UI input meter. Post-chain meter is on the engine
  output.

### 9.2 Clipper / Distortion
- `drive`: 1.0–10.0 (pre-gain).
- `hardClip`: 0.0–1.0 (threshold; samples above are clamped).
- `softClip`: 0.0–1.0 (tanh blend amount).
- `outputTrimDb`: −12 to +6 dB (post-stage trim).
- Final limiter (9.8) always runs after, regardless of drive setting.

### 9.3 Bitcrusher
- `bitDepth`: 1–16 bits (quantize amplitude).
- `sampleRateHz`: 1000–48000 Hz (sample-and-hold downsampling).
- `mix`: 0.0–1.0 wet/dry.

### 9.4 Bandpass / Telephone EQ
- `lowCutHz`: 20–8000 Hz (high-pass biquad).
- `highCutHz`: 200–20000 Hz (low-pass biquad).
- `midBoostDb`: −12 to +12 dB (peaking biquad at midpoint of band).
- Telephone / cheap headset / radio character is achieved via **presets**,
  not separate effect modes.

### 9.5 Noise Generator
- `whiteAmount`: 0.0–1.0.
- `humAmount`: 0.0–1.0 at `humHz` (50 or 60).
- `crackleRate`: 0–20 events/sec.
- `gateMode`: `Always` | `OnSpeech` (side-chain detector on input level).
- **Pink noise** dropped from MVP (white only).

### 9.6 Packet Loss / Dropout
- `dropChance`: 0.0–0.5 (per evaluation window).
- `minDropMs` / `maxDropMs`: 10–500 ms drop length.
- `stutterChance`: 0.0–0.3 (repeat last ~40 ms instead of dropping).
- All transitions ramp over 2–5 ms to avoid clicks.
- "Jitter/robotic chunking" implemented as small random repeats of the prior
  chunk during dropouts.

### 9.7 Bad Noise Gate
- `thresholdDb`: −60 to 0 dB.
- `attackMs`: 0–200 ms.
- `releaseMs`: 0–500 ms.
- `chatterAmount`: 0.0–1.0 (random false-close probability per 10 ms window
  while signal is just above threshold).

### 9.8 Final Limiter
- `ceilingDb`: −24 to 0 dB (default −3 dB).
- `releaseMs`: 1–500 ms (default 80 ms).
- **Peak limiter without lookahead** — 5 ms attack, configurable release.
  Sufficient for ear-safety; no added latency.
- Fixed position at end of chain. Cannot be reordered out.
- "Disable limiter" requires Safe Output Mode = off + confirmation dialog.

## 10. Main UI

```
┌──────────────────────────────────────────────────────────────┐
│  [Input: USB Mic ▾]   [Output: CABLE Input ▾]   ⚠ health    │  ← top bar
├──────┬──────────────────────────────────┬────────────────────┤
│      │                                  │                    │
│ Pre- │      Effect Chain                │  Selected Effect   │
│ sets │   ┌────────────────┐             │  Parameters        │
│      │   │ ☑ Gain          │ ←─── drag  │                    │
│ Xbox │   │ ☑ Bandpass      │            │  drive   [====]    │
│ 360  │   │ ☑ Bitcrusher    │            │  hard    [==  ]    │
│      │   │ ☑ Clipper       │            │  soft    [=   ]    │
│ Tin  │   │ ☐ Noise         │            │  trim    [==  ]    │
│ Can  │   │ ☑ Packet Loss   │            │                    │
│      │   │ ☑ Limiter (fix) │            │                    │
│ ...  │   └────────────────┘             │                    │
│      │   [+ Add Effect]                 │                    │
├──────┴──────────────────────────────────┴────────────────────┤
│ Input ████░░  Out ███░░░  ⚠ LIM  [ START ]  [● Test Record] │  ← bottom
└──────────────────────────────────────────────────────────────┘
```

- Dark theme. Small rat iconography in titlebar / about screen. No meme
  overload in the chain UI itself.
- Setup help screen (modal): 4 steps for VB-CABLE flow.
- Routing health panel: list of advisory checks with ✅/⚠️/❌.

## 11. Implementation Phases

| # | Goal | Verify |
|---|---|---|
| 0 | Tauri+Svelte skeleton, settings round-trip | App launches, writes/reads `%APPDATA%\RatMic\settings.json` |
| 1 | Device enum + cpal passthrough (identity DSP), meters | Speaking into mic produces sound on selected output device, meters move, ≤ 30 ms RTT |
| 2 | Effect chain framework, click-free toggle, fixed limiter | Toggling an identity-pass effect has no audible click; limiter blocks > -3 dB |
| 3 | All 8 MVP effects + per-effect param UI | Each effect changes the audio audibly and serializes round-trip |
| 4 | Hotkey (hold + toggle), local monitor, 5-sec test record | Hotkey activates RatMic from Discord-focused window; test record produces a playable WAV of processed audio |
| 5 | Built-in + user presets, import/export, schema validation | All 10 built-ins load; user preset survives app restart; corrupt JSON fails safely |
| 6 | Routing health, disconnect handling, Safe Output Mode enforcement | Unplug mic mid-session → warning + auto-stop; output ceiling hard-clamped in safe mode |
| 7 | Full manual test pass | Checklist in §14 all green |

Estimated 10–14 working days for one engineer.

## 12. Risks & Mitigations

| # | Risk | Mitigation |
|---|---|---|
| R1 | WASAPI shared-mode latency floor (~15–25 ms) — user expects "imperceptible" | Document expected RTT in setup screen. If unacceptable, fall back to direct `wasapi` event-driven mode (v2). |
| R2 | Lookahead limiter adds latency without proportional safety benefit | Use peak limiter w/o lookahead. -3 dB ceiling + 5 ms attack is enough to prevent ear damage. |
| R3 | Click-free toggle is non-obvious | Every effect bypass + chain swap uses ≥ 5 ms crossfade ramp. Tested in Phase 2. |
| R4 | Global hotkey misses keys in DirectInput-exclusive fullscreen | Known v1 limitation; documented. Raw-input hook is v2. |
| R5 | Tauri IPC throughput | Meter updates aggregated to ~16 ms tick rate in worker. No per-sample data over IPC ever. |
| R6 | Cursed defaults cause ear damage | Safe Output Mode (default on) hard-clamps noise amount, drive, ceiling. Disabling needs confirm. |
| R7 | Memory growth over long sessions | All hot-path buffers preallocated. Test recording uses fixed circular buffer. CI smoke test runs engine 30 min idle. |
| R8 | cpal lacks native device-change events on Windows | Device-watch timer polls every 2 s, surfaces disconnect as non-fatal warning + auto-stop. |
| R9 | Tauri + serious audio is uncommon | egui is a clean fallback (audio engine unchanged). Don't switch unless we hit a real wall. |
| R10 | Sample-rate mismatch between devices | Explicit `rubato` stages at I/O boundaries. Internal pipeline is fixed 48 kHz. |
| R11 | Preset schema evolves and breaks old user files | `schema_version` field on every preset and settings file. Migration step at load. |

## 13. Cuttable Scope (drop if time-pressured)

These are in MVP but **not load-bearing** to the product identity:

- Bandpass mid-boost peak → just LP + HP biquads.
- "Noise only while speaking" → constant noise.
- Stutter/repeat in packet loss → just dropouts.
- `chatterAmount` in noise gate → simple threshold gate.

The character of the 10 built-in presets carries the "bad mic" feel even
without these.

## 14. Testing Plan (manual)

Tracked in `tests/manual_checklist.md`. Categories:

**Device**
- [ ] No mic connected → friendly error, app does not crash
- [ ] Mic connected after app starts → appears in dropdown without restart
- [ ] Output device disconnected mid-session → warning + auto-stop
- [ ] Input/output device changed while running → engine restarts cleanly
- [ ] Input device == output device → blocking warning
- [ ] VB-CABLE missing → output list lacks it, app still works with regular outputs

**Audio**
- [ ] No clipping with default presets
- [ ] Limiter activates correctly (verifiable via clip-warning indicator)
- [ ] No pops when toggling individual effects
- [ ] No pops when switching presets
- [ ] No huge volume spikes during preset change
- [ ] Bitcrusher does not crash at min bit-depth / min sample-rate
- [ ] Packet loss does not produce sharp clicks
- [ ] Noise generator cannot exceed −3 dB peak in safe mode
- [ ] Local monitor does not feedback at default settings

**Discord integration**
- [ ] RatMic output → VB-CABLE Input; Discord input = VB-CABLE Output → Discord hears processed voice
- [ ] Discord mic test reflects processed audio
- [ ] Push-to-enable hotkey works while Discord is focused window
- [ ] 30+ minute session: no audio glitches, no memory growth, no UI lock

**Preset**
- [ ] Each of 10 built-in presets loads
- [ ] Save user preset → restart app → preset survives
- [ ] Export preset → import elsewhere → identical audio
- [ ] Invalid preset JSON → user-visible error, no crash, file untouched
- [ ] Missing effect type in preset → skipped with warning

**Performance**
- [ ] CPU < 1% idle (engine stopped)
- [ ] CPU < 5% with all 8 effects active on a modern CPU
- [ ] RTT ≤ 30 ms with default cpal buffer size
- [ ] Memory stable across 1-hour run (within 10 MB drift)

**Boot-time / robustness**
- [ ] Cold start to ready ≤ 2 s
- [ ] Boot with saved input device missing → falls back to default + warning
- [ ] Hand-corrupted `settings.json` → boots with defaults + visible warning
- [ ] App update preserves user presets

## 15. Safety Requirements

- Final limiter is fixed at the end of the chain and cannot be reordered.
- Safe Output Mode (default on) hard-clamps:
  - Limiter ceiling ≥ −3 dB
  - Noise white amount ≤ 0.5
  - Clipper drive ≤ 5.0
- Smooth parameter changes: every numeric param ramps to its new value over
  ~10 ms in the audio thread.
- Clip warning indicator triggers if any sample reaches > −0.5 dB at output
  (separate from limiter activity indicator).
- Default presets are calibrated to be funny, not loud. None should peg
  the limiter on normal speech.
- All effect parameter inputs in the UI are bounded by the param's documented
  range (no free-text float entry).

## 16. File / Folder Structure

```
ratmic/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs                # bootstrap + command registration
│       ├── commands.rs            # IPC handlers
│       ├── events.rs              # event types (meters, warnings)
│       ├── settings.rs            # %APPDATA% persistence
│       ├── test_record.rs         # 5-sec output capture → WAV
│       ├── audio/
│       │   ├── mod.rs
│       │   ├── engine.rs          # orchestrates I/O + worker
│       │   ├── devices.rs         # enumeration + persistent IDs
│       │   ├── input_stream.rs    # cpal input wrapper
│       │   ├── output_backend.rs  # AudioOutputBackend trait
│       │   ├── system_output.rs   # v1 backend impl
│       │   ├── monitor.rs         # local monitor stream
│       │   ├── resampler.rs       # rubato wrapper
│       │   ├── ring_buffer.rs     # typed ringbuf wrapper
│       │   ├── meters.rs          # peak/RMS detector
│       │   └── format.rs          # sample format helpers
│       ├── effects/
│       │   ├── mod.rs             # Effect trait + registry
│       │   ├── chain.rs           # chain processor + crossfade
│       │   ├── biquad.rs          # shared filter primitive
│       │   ├── gain.rs
│       │   ├── clipper.rs
│       │   ├── bitcrusher.rs
│       │   ├── bandpass.rs
│       │   ├── noise.rs
│       │   ├── packet_loss.rs
│       │   ├── noise_gate.rs
│       │   └── limiter.rs
│       ├── presets/
│       │   ├── mod.rs
│       │   ├── schema.rs          # versioned types + validation
│       │   ├── builtin.rs         # include_str! of bundled JSONs
│       │   ├── user.rs            # user preset save/load
│       │   └── builtin_json/      # 10 built-in .json files
│       ├── hotkeys/
│       │   ├── mod.rs
│       │   └── manager.rs
│       └── diagnostics/
│           ├── mod.rs
│           ├── routing_check.rs   # device name heuristics
│           └── stats.rs           # clipping/limiter activity
├── src/                            # Svelte frontend
│   ├── App.svelte
│   ├── main.ts
│   └── lib/
│       ├── ipc.ts                  # typed Tauri command wrappers
│       ├── stores.ts               # Svelte stores
│       └── components/
│           ├── DeviceBar.svelte
│           ├── PresetSidebar.svelte
│           ├── EffectChain.svelte
│           ├── EffectParams.svelte
│           ├── MeterBar.svelte
│           ├── SetupHelp.svelte
│           └── RoutingHealth.svelte
├── docs/
│   └── superpowers/specs/2026-05-24-ratmic-design.md
├── tests/manual_checklist.md
├── package.json
└── README.md
```

## 17. Built-in Presets (v1)

Ship 10 JSON files in `src-tauri/src/presets/builtin_json/`:

1. Xbox 360 Lobby
2. Cheap Headset
3. Drive-Thru Speaker
4. Broken Radio
5. Discord Packet Loss
6. Deep Fried Mic
7. Tin Can
8. Underwater
9. Fan Noise Hell
10. 2007 Skype Call

Each preset must:
- Pass schema validation under Safe Output Mode (i.e., no params exceeding
  safe-mode clamps).
- Sound recognizably different from the others on default speech input.
- Not peg the final limiter on normal-volume speech.

## 18. Open Questions

None blocking. Items to revisit if surfaced during implementation:

- If cpal RTT consistently exceeds 30 ms, consider direct `wasapi` event-driven
  mode (Phase 1 verify step gates this).
- If Tauri↔Svelte IPC churn becomes painful, fall back to egui (R9).
- If users report missed hotkeys in fullscreen-exclusive games, raw-input
  hook moves up from v2 to v1.5.

---

## Approval

Approved verbally 2026-05-24. Proceeding to implementation plan via
`writing-plans` skill.
