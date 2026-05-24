# RatMic Effects Implementation Plan (Phase 3 — remaining MVP effects)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the seven remaining MVP effects (Clipper, Bitcrusher, Bandpass, Noise, Packet Loss, Bad Noise Gate) plus a shared Biquad primitive, wire them all into the default audio chain, and add per-effect parameter editors in the UI — so the user can hear the full "bad mic" character without leaving the app.

**Architecture:** Each effect is a self-contained Rust module under `src-tauri/src/effects/` implementing the existing `Effect` trait. They share a small Biquad primitive (RBJ cookbook). Parameters are validated and clamped on `set_params`. The default chain in `AudioEngine::start` is extended to include all eight effects in a sensible order, all disabled by default except the fixed final Limiter. Frontend gets one focused `<EffectEditor>` component per effect type, dispatched from `EffectParams.svelte`.

**Tech Stack:** Same as foundation plan (Tauri 2, Svelte 4 + TS, Rust + cpal). One new Rust dep: `fastrand` 2.x for noise and dropout RNG.

**User preference:** No `git init`, no commits during the plan unless explicitly asked.

## Known Deferrals

These come in later plans, not this one:
- **Add/remove/reorder effects from the UI.** Plan 4 (presets) handles dynamic chain editing.
- **Internal-SR resampling.** Still deferred to Plan 5; if input/output device SRs differ, things will sound off.
- **Hotkey, local monitor, test recording, routing health, Safe Output Mode clamps.** Plans 3 & 5.

---

## File Structure Map

| Task | File |
|---|---|
| 1 | `src-tauri/Cargo.toml` (add `fastrand`), `src-tauri/src/effects/biquad.rs` (new) |
| 1 | `src-tauri/src/effects/mod.rs` (register `biquad`) |
| 2 | `src-tauri/src/effects/clipper.rs` (new) |
| 3 | `src-tauri/src/effects/bitcrusher.rs` (new) |
| 4 | `src-tauri/src/effects/bandpass.rs` (new) |
| 5 | `src-tauri/src/effects/noise.rs` (new) |
| 6 | `src-tauri/src/effects/packet_loss.rs` (new) |
| 7 | `src-tauri/src/effects/noise_gate.rs` (new) |
| 8 | `src-tauri/src/effects/mod.rs` (register all 6 new modules) |
| 8 | `src-tauri/src/audio/engine.rs` (extend default chain) |
| 9 | `src/lib/components/effects/*Editor.svelte` (8 new files: one per effect) |
| 9 | `src/lib/components/EffectParams.svelte` (refactored to dispatch) |
| 10 | Manual smoke test |

---

## Task 1: Biquad primitive + fastrand dep

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\Cargo.toml`
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\biquad.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

A biquad filter is the standard building block for parametric EQ (Bandpass effect, future effects). Implementation follows the Robert Bristow-Johnson (RBJ) audio cookbook for LPF, HPF, and peak EQ.

- [ ] **Step 1: Add `fastrand` to `Cargo.toml`**

Edit `src-tauri/Cargo.toml`, append to `[dependencies]`:

```toml
fastrand = "2"
```

- [ ] **Step 2: Verify dep resolves**

Run: `cd src-tauri && cargo check`
Expected: clean compile.

- [ ] **Step 3: Register biquad module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod biquad;` next to the existing `pub mod` declarations (preserving alphabetical order if used).

- [ ] **Step 4: Write the Biquad module with tests**

Create `src-tauri/src/effects/biquad.rs`:

```rust
//! RBJ-cookbook biquad filter.
//!
//! Direct form I, single-sample-per-call. State: x[n-1], x[n-2], y[n-1], y[n-2].
//!
//! Coefficients are pre-normalized by a0 so the runtime loop is fewer ops.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub enum FilterKind {
    LowPass,
    HighPass,
    PeakEq { gain_db: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoefs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BiquadCoefs {
    pub fn identity() -> Self {
        Self { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 }
    }

    pub fn design(kind: FilterKind, freq_hz: f32, q: f32, sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        // Clamp frequency to a safe range to avoid pathological coefficients.
        let f0 = freq_hz.clamp(10.0, sr * 0.45);
        let q = q.max(0.001);
        let w0 = 2.0 * PI * f0 / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            FilterKind::LowPass => {
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterKind::HighPass => {
                let b1 = -(1.0 + cos_w0);
                let b0 = (1.0 + cos_w0) / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterKind::PeakEq { gain_db } => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                (
                    1.0 + alpha * a,
                    -2.0 * cos_w0,
                    1.0 - alpha * a,
                    1.0 + alpha / a,
                    -2.0 * cos_w0,
                    1.0 - alpha / a,
                )
            }
        };

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    pub coefs: BiquadCoefs,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new(coefs: BiquadCoefs) -> Self {
        Self { coefs, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    pub fn identity() -> Self {
        Self::new(BiquadCoefs::identity())
    }

    pub fn set_coefs(&mut self, coefs: BiquadCoefs) {
        self.coefs = coefs;
        // Don't clear state — coefficient changes mid-stream should be smooth.
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    pub fn process_sample(&mut self, x: f32) -> f32 {
        let c = &self.coefs;
        let y = c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2
            - c.a1 * self.y1 - c.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    pub fn process_buffer(&mut self, buffer: &mut [f32]) {
        for s in buffer {
            *s = self.process_sample(*s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a biquad with a sine of given frequency and return peak amplitude
    /// of the steady-state output (after a warmup).
    fn measure_response(b: &mut Biquad, freq_hz: f32, sample_rate: u32, samples: usize) -> f32 {
        let sr = sample_rate as f32;
        // Warm up so initial transient passes.
        for n in 0..2048 {
            let t = n as f32 / sr;
            let x = (2.0 * PI * freq_hz * t).sin();
            b.process_sample(x);
        }
        let mut peak = 0.0_f32;
        for n in 0..samples {
            let t = n as f32 / sr;
            let x = (2.0 * PI * freq_hz * t).sin();
            let y = b.process_sample(x);
            peak = peak.max(y.abs());
        }
        peak
    }

    #[test]
    fn identity_passes_signal() {
        let mut b = Biquad::identity();
        let mut buf = vec![0.1, 0.2, -0.5, 0.7];
        b.process_buffer(&mut buf);
        // Identity should leave samples close to original (no perfect equality due to
        // the recursive form, but it should not move them by more than ~1e-6).
        let expected = [0.1, 0.2, -0.5, 0.7];
        for (a, e) in buf.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-6, "identity drifted: {} vs {}", a, e);
        }
    }

    #[test]
    fn lowpass_attenuates_high_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 500.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let high = measure_response(&mut b, 8000.0, 48000, 4096);
        // 8 kHz is well above 500 Hz cutoff — expect heavy attenuation.
        assert!(high < 0.1, "8 kHz peak should be < 0.1, got {}", high);
    }

    #[test]
    fn lowpass_passes_low_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 2000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let low = measure_response(&mut b, 200.0, 48000, 4096);
        // 200 Hz is well below 2 kHz cutoff — should pass near unity.
        assert!(low > 0.9, "200 Hz peak should be > 0.9, got {}", low);
    }

    #[test]
    fn highpass_attenuates_low_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::HighPass, 1000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let low = measure_response(&mut b, 100.0, 48000, 4096);
        assert!(low < 0.1, "100 Hz peak should be < 0.1, got {}", low);
    }

    #[test]
    fn highpass_passes_high_frequency() {
        let coefs = BiquadCoefs::design(FilterKind::HighPass, 500.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        let high = measure_response(&mut b, 6000.0, 48000, 4096);
        assert!(high > 0.9, "6 kHz peak should be > 0.9, got {}", high);
    }

    #[test]
    fn peak_eq_boosts_center_frequency() {
        let coefs = BiquadCoefs::design(
            FilterKind::PeakEq { gain_db: 12.0 },
            1000.0,
            1.0,
            48000,
        );
        let mut b = Biquad::new(coefs);
        let center = measure_response(&mut b, 1000.0, 48000, 4096);
        // +12 dB is ~3.98x amplitude.
        assert!(center > 3.0, "1 kHz peak with +12 dB should be > 3.0, got {}", center);
    }

    #[test]
    fn reset_clears_state() {
        let coefs = BiquadCoefs::design(FilterKind::LowPass, 1000.0, 0.707, 48000);
        let mut b = Biquad::new(coefs);
        b.process_sample(1.0);
        b.process_sample(1.0);
        assert_ne!(b.y1, 0.0);
        b.reset();
        assert_eq!(b.y1, 0.0);
        assert_eq!(b.y2, 0.0);
        assert_eq!(b.x1, 0.0);
        assert_eq!(b.x2, 0.0);
    }
}
```

- [ ] **Step 5: Run biquad tests**

Run: `cd src-tauri && cargo test effects::biquad`
Expected: 7 tests pass.

---

## Task 2: Clipper effect

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\clipper.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

Hard + soft clipping with pre-gain drive and post-stage trim. Order inside the effect: input → drive → soft clip (tanh blend) → hard clip (clamp) → trim.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod clipper;`.

- [ ] **Step 2: Implement Clipper with tests**

Create `src-tauri/src/effects/clipper.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::clipper`
Expected: 6 tests pass.

---

## Task 3: Bitcrusher effect

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\bitcrusher.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

Bit-depth quantization + sample-and-hold downsampling, blended with the dry signal.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod bitcrusher;`.

- [ ] **Step 2: Implement Bitcrusher with tests**

Create `src-tauri/src/effects/bitcrusher.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::bitcrusher`
Expected: 5 tests pass.

---

## Task 4: Bandpass effect

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\bandpass.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

High-pass biquad (`lowCutHz`) → low-pass biquad (`highCutHz`) → peak biquad (`midBoostDb` at midpoint).

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod bandpass;`.

- [ ] **Step 2: Implement Bandpass with tests**

Create `src-tauri/src/effects/bandpass.rs`:

```rust
//! Bandpass + optional mid-boost: HP → LP → peak EQ.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::biquad::{Biquad, BiquadCoefs, FilterKind};
use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandpassParams {
    #[serde(rename = "lowCutHz", default = "default_low_cut")]
    pub low_cut_hz: f32,
    #[serde(rename = "highCutHz", default = "default_high_cut")]
    pub high_cut_hz: f32,
    #[serde(rename = "midBoostDb", default)]
    pub mid_boost_db: f32,
}

fn default_low_cut() -> f32 { 100.0 }
fn default_high_cut() -> f32 { 8000.0 }

impl Default for BandpassParams {
    fn default() -> Self {
        Self {
            low_cut_hz: default_low_cut(),
            high_cut_hz: default_high_cut(),
            mid_boost_db: 0.0,
        }
    }
}

const MIN_LOW: f32 = 20.0;
const MAX_LOW: f32 = 8000.0;
const MIN_HIGH: f32 = 200.0;
const MAX_HIGH: f32 = 20_000.0;
const MIN_BOOST_DB: f32 = -12.0;
const MAX_BOOST_DB: f32 = 12.0;

pub struct Bandpass {
    params: BandpassParams,
    sample_rate: u32,
    hp: Biquad,
    lp: Biquad,
    peak: Biquad,
}

impl Bandpass {
    pub fn new(sample_rate: u32, params: BandpassParams) -> Self {
        let mut b = Self {
            params,
            sample_rate,
            hp: Biquad::identity(),
            lp: Biquad::identity(),
            peak: Biquad::identity(),
        };
        b.recompute();
        b
    }

    fn recompute(&mut self) {
        let hp = BiquadCoefs::design(FilterKind::HighPass, self.params.low_cut_hz, 0.707, self.sample_rate);
        let lp = BiquadCoefs::design(FilterKind::LowPass, self.params.high_cut_hz, 0.707, self.sample_rate);
        self.hp.set_coefs(hp);
        self.lp.set_coefs(lp);
        if self.params.mid_boost_db.abs() < 0.01 {
            self.peak.set_coefs(BiquadCoefs::identity());
        } else {
            let mid = (self.params.low_cut_hz * self.params.high_cut_hz).sqrt();
            let peak = BiquadCoefs::design(
                FilterKind::PeakEq { gain_db: self.params.mid_boost_db },
                mid, 1.0, self.sample_rate,
            );
            self.peak.set_coefs(peak);
        }
    }
}

impl Effect for Bandpass {
    fn type_name(&self) -> &'static str { "bandpass" }

    fn process(&mut self, buffer: &mut [f32]) {
        for s in buffer {
            let mut y = self.hp.process_sample(*s);
            y = self.lp.process_sample(y);
            y = self.peak.process_sample(y);
            *s = y;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: BandpassParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.low_cut_hz = p.low_cut_hz.clamp(MIN_LOW, MAX_LOW);
        p.high_cut_hz = p.high_cut_hz.clamp(MIN_HIGH, MAX_HIGH);
        // Ensure low < high.
        if p.low_cut_hz >= p.high_cut_hz {
            p.high_cut_hz = (p.low_cut_hz + 100.0).min(MAX_HIGH);
        }
        p.mid_boost_db = p.mid_boost_db.clamp(MIN_BOOST_DB, MAX_BOOST_DB);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("bandpass params serialize")
    }

    fn reset(&mut self) {
        self.hp.reset();
        self.lp.reset();
        self.peak.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn measure_peak(b: &mut Bandpass, freq_hz: f32) -> f32 {
        // Continuous phase across warmup + measurement to avoid a discontinuity
        // at the boundary (which manifests as a fake "peak" dominating the read).
        const WARMUP: usize = 4096;
        const MEASURE: usize = 4096;
        let mut peak = 0.0_f32;
        for n in 0..(WARMUP + MEASURE) {
            let t = n as f32 / 48000.0;
            let x = (2.0 * PI * freq_hz * t).sin();
            let mut buf = [x];
            b.process(&mut buf);
            if n >= WARMUP {
                peak = peak.max(buf[0].abs());
            }
        }
        peak
    }

    #[test]
    fn telephone_band_passes_mid_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let mid = measure_peak(&mut b, 1000.0);
        assert!(mid > 0.7, "1 kHz should pass, got {}", mid);
    }

    #[test]
    fn telephone_band_attenuates_low_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let low = measure_peak(&mut b, 50.0);
        assert!(low < 0.2, "50 Hz should be attenuated, got {}", low);
    }

    #[test]
    fn telephone_band_attenuates_high_frequencies() {
        let mut b = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let high = measure_peak(&mut b, 12000.0);
        assert!(high < 0.2, "12 kHz should be attenuated, got {}", high);
    }

    #[test]
    fn mid_boost_amplifies_band_center() {
        let mut flat = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 0.0,
        });
        let mut boosted = Bandpass::new(48000, BandpassParams {
            low_cut_hz: 300.0,
            high_cut_hz: 3400.0,
            mid_boost_db: 9.0,
        });
        // Geometric mean of 300 & 3400 is ~1010 Hz.
        let flat_peak = measure_peak(&mut flat, 1010.0);
        let boosted_peak = measure_peak(&mut boosted, 1010.0);
        assert!(
            boosted_peak > flat_peak * 1.5,
            "+9 dB boost should be > 1.5x flat (got {} vs {})",
            boosted_peak, flat_peak
        );
    }

    #[test]
    fn params_clamp_and_enforce_low_lt_high() {
        let mut b = Bandpass::new(48000, BandpassParams::default());
        b.set_params(&serde_json::json!({
            "lowCutHz": 5000.0,
            "highCutHz": 2000.0,
            "midBoostDb": 50.0
        })).unwrap();
        let p: BandpassParams = serde_json::from_value(b.get_params()).unwrap();
        assert!(p.low_cut_hz < p.high_cut_hz);
        assert_eq!(p.mid_boost_db, MAX_BOOST_DB);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::bandpass`
Expected: 5 tests pass.

---

## Task 5: Noise generator

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\noise.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

White noise + 50/60 Hz hum + crackle bursts. Optional "on speech only" mode using input level as a gate.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod noise;`.

- [ ] **Step 2: Implement Noise with tests**

Create `src-tauri/src/effects/noise.rs`:

```rust
//! White noise + hum + crackle, optionally gated by input level.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use std::f32::consts::PI;

use super::Effect;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NoiseGateMode {
    Always,
    OnSpeech,
}

impl Default for NoiseGateMode {
    fn default() -> Self { Self::Always }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseParams {
    #[serde(rename = "whiteAmount", default)]
    pub white_amount: f32,
    #[serde(rename = "humAmount", default)]
    pub hum_amount: f32,
    #[serde(rename = "humHz", default = "default_hum_hz")]
    pub hum_hz: f32,
    #[serde(rename = "crackleRate", default)]
    pub crackle_rate: f32, // events per second
    #[serde(rename = "gateMode", default)]
    pub gate_mode: NoiseGateMode,
    #[serde(rename = "speechThresholdDb", default = "default_speech_thresh")]
    pub speech_threshold_db: f32,
}

fn default_hum_hz() -> f32 { 60.0 }
fn default_speech_thresh() -> f32 { -40.0 }

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            white_amount: 0.0,
            hum_amount: 0.0,
            hum_hz: default_hum_hz(),
            crackle_rate: 0.0,
            gate_mode: NoiseGateMode::Always,
            speech_threshold_db: default_speech_thresh(),
        }
    }
}

const MAX_WHITE: f32 = 0.5;
const MAX_HUM: f32 = 0.5;
const MIN_HUM_HZ: f32 = 40.0;
const MAX_HUM_HZ: f32 = 120.0;
const MAX_CRACKLE_RATE: f32 = 50.0;

pub struct Noise {
    params: NoiseParams,
    sample_rate: u32,
    rng: fastrand::Rng,
    /// Hum phase accumulator, radians.
    hum_phase: f32,
    /// Smoothed input envelope for gate-mode decisions.
    envelope: f32,
    env_attack: f32,
    env_release: f32,
}

impl Noise {
    pub fn new(sample_rate: u32, params: NoiseParams) -> Self {
        let sr = sample_rate as f32;
        // ~5 ms attack, ~80 ms release on the envelope follower.
        let env_attack = (-1.0 / (0.005 * sr)).exp();
        let env_release = (-1.0 / (0.080 * sr)).exp();
        Self {
            params,
            sample_rate,
            rng: fastrand::Rng::new(),
            hum_phase: 0.0,
            envelope: 0.0,
            env_attack,
            env_release,
        }
    }

    fn next_white(&mut self) -> f32 {
        // Uniform in [-1.0, 1.0)
        self.rng.f32() * 2.0 - 1.0
    }

    fn maybe_crackle(&mut self) -> f32 {
        let rate = self.params.crackle_rate;
        if rate <= 0.0 { return 0.0; }
        let p = rate / self.sample_rate as f32;
        if self.rng.f32() < p {
            // Short impulse, +/- random amplitude in [0.3, 0.8].
            let amp = 0.3 + self.rng.f32() * 0.5;
            if self.rng.bool() { amp } else { -amp }
        } else {
            0.0
        }
    }
}

impl Effect for Noise {
    fn type_name(&self) -> &'static str { "noise" }

    fn process(&mut self, buffer: &mut [f32]) {
        let white = self.params.white_amount.clamp(0.0, MAX_WHITE);
        let hum = self.params.hum_amount.clamp(0.0, MAX_HUM);
        let hum_hz = self.params.hum_hz.clamp(MIN_HUM_HZ, MAX_HUM_HZ);
        let phase_inc = 2.0 * PI * hum_hz / self.sample_rate as f32;
        let speech_thresh_amp = 10.0_f32.powf(self.params.speech_threshold_db / 20.0);
        let gate_on_speech = matches!(self.params.gate_mode, NoiseGateMode::OnSpeech);
        let env_attack = self.env_attack;
        let env_release = self.env_release;

        for s in buffer {
            let abs = s.abs();
            // Envelope follower (5 ms attack / 80 ms release, set in new()).
            let coef = if abs > self.envelope { env_attack } else { env_release };
            self.envelope = abs + (self.envelope - abs) * coef;

            // Gate decision.
            let gate_open = !gate_on_speech || self.envelope > speech_thresh_amp;

            let mut noise = 0.0;
            if gate_open {
                if white > 0.0 {
                    noise += self.next_white() * white;
                }
                if hum > 0.0 {
                    noise += self.hum_phase.sin() * hum;
                }
                noise += self.maybe_crackle();
            }

            self.hum_phase += phase_inc;
            if self.hum_phase > 2.0 * PI {
                self.hum_phase -= 2.0 * PI;
            }

            *s += noise;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: NoiseParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.white_amount = p.white_amount.clamp(0.0, MAX_WHITE);
        p.hum_amount = p.hum_amount.clamp(0.0, MAX_HUM);
        p.hum_hz = p.hum_hz.clamp(MIN_HUM_HZ, MAX_HUM_HZ);
        p.crackle_rate = p.crackle_rate.clamp(0.0, MAX_CRACKLE_RATE);
        p.speech_threshold_db = p.speech_threshold_db.clamp(-90.0, 0.0);
        self.params = p;
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("noise params serialize")
    }

    fn reset(&mut self) {
        self.hum_phase = 0.0;
        self.envelope = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_params_is_identity() {
        let mut n = Noise::new(48000, NoiseParams::default());
        let mut buf = vec![0.1, 0.2, -0.3];
        n.process(&mut buf);
        // No noise added → output equals input.
        assert!((buf[0] - 0.1).abs() < 1e-5);
        assert!((buf[1] - 0.2).abs() < 1e-5);
        assert!((buf[2] - (-0.3)).abs() < 1e-5);
    }

    #[test]
    fn white_amount_adds_audible_noise() {
        let mut n = Noise::new(48000, NoiseParams {
            white_amount: 0.3,
            ..Default::default()
        });
        let mut buf = vec![0.0; 1000];
        n.process(&mut buf);
        let max = buf.iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(max > 0.05, "white noise should produce signal, max {}", max);
    }

    #[test]
    fn hum_produces_periodic_signal() {
        let mut n = Noise::new(48000, NoiseParams {
            hum_amount: 0.3,
            hum_hz: 60.0,
            ..Default::default()
        });
        let mut buf = vec![0.0; 4800]; // 100 ms @ 48 kHz
        n.process(&mut buf);
        // 60 Hz over 100 ms = 6 cycles. Count zero crossings — expect ~12 (2 per cycle).
        let mut crossings = 0;
        for w in buf.windows(2) {
            if w[0].signum() != w[1].signum() && w[0] != 0.0 {
                crossings += 1;
            }
        }
        assert!((10..=14).contains(&crossings), "expected ~12 crossings, got {}", crossings);
    }

    #[test]
    fn on_speech_gates_when_input_quiet() {
        let mut n = Noise::new(48000, NoiseParams {
            white_amount: 0.5,
            gate_mode: NoiseGateMode::OnSpeech,
            speech_threshold_db: -30.0,
            ..Default::default()
        });
        // All-zero input → noise should be gated off (after envelope decays).
        let mut buf = vec![0.0; 4800];
        n.process(&mut buf);
        // After envelope has settled, late samples should be zero.
        let tail_max = buf[3000..].iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(tail_max < 0.01, "gated noise should be silent in tail, got {}", tail_max);
    }

    #[test]
    fn params_clamp_white_to_safe_range() {
        let mut n = Noise::new(48000, NoiseParams::default());
        n.set_params(&serde_json::json!({ "whiteAmount": 10.0 })).unwrap();
        let p: NoiseParams = serde_json::from_value(n.get_params()).unwrap();
        assert_eq!(p.white_amount, MAX_WHITE);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::noise`
Expected: 5 tests pass.

---

## Task 6: Packet loss / dropout

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\packet_loss.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

Random dropouts with optional stutter (replay a short circular buffer of recent audio) and 2 ms fade in/out to avoid clicks.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod packet_loss;`.

- [ ] **Step 2: Implement PacketLoss with tests**

Create `src-tauri/src/effects/packet_loss.rs`:

```rust
//! Packet loss / dropout simulator with optional stutter and fade ramps.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketLossParams {
    #[serde(rename = "dropChance", default)]
    pub drop_chance: f32,
    #[serde(rename = "minDropMs", default = "default_min_drop")]
    pub min_drop_ms: f32,
    #[serde(rename = "maxDropMs", default = "default_max_drop")]
    pub max_drop_ms: f32,
    #[serde(rename = "stutterChance", default)]
    pub stutter_chance: f32,
}

fn default_min_drop() -> f32 { 30.0 }
fn default_max_drop() -> f32 { 140.0 }

impl Default for PacketLossParams {
    fn default() -> Self {
        Self {
            drop_chance: 0.0,
            min_drop_ms: default_min_drop(),
            max_drop_ms: default_max_drop(),
            stutter_chance: 0.0,
        }
    }
}

const MAX_DROP_CHANCE: f32 = 0.5;
const MIN_DROP_MS: f32 = 10.0;
const MAX_DROP_MS: f32 = 500.0;
const MAX_STUTTER_CHANCE: f32 = 0.3;
const FADE_MS: f32 = 2.0;
const HISTORY_MS: f32 = 80.0;
/// How often (in samples) we evaluate "should a new drop start?". 20 ms cadence
/// at 48 kHz = 960 samples.
const EVAL_INTERVAL_MS: f32 = 20.0;

#[derive(Debug, Clone, Copy)]
enum DropMode {
    Silent,
    Stutter,
}

#[derive(Debug, Clone, Copy)]
enum State {
    PassThrough,
    FadingOut { remaining: u32, total: u32, mode: DropMode },
    Dropped { remaining: u32, mode: DropMode },
    FadingIn { remaining: u32, total: u32, mode: DropMode },
}

pub struct PacketLoss {
    params: PacketLossParams,
    sample_rate: u32,
    rng: fastrand::Rng,
    state: State,
    eval_counter: u32,
    eval_period: u32,
    fade_samples: u32,
    history: Vec<f32>,
    history_write: usize,
    /// During stutter, where we read from history.
    stutter_read: usize,
}

impl PacketLoss {
    pub fn new(sample_rate: u32, params: PacketLossParams) -> Self {
        let sr = sample_rate as f32;
        let eval_period = ((EVAL_INTERVAL_MS * 0.001) * sr) as u32;
        let fade_samples = ((FADE_MS * 0.001) * sr) as u32;
        let history_len = ((HISTORY_MS * 0.001) * sr) as usize;
        Self {
            params,
            sample_rate,
            rng: fastrand::Rng::new(),
            state: State::PassThrough,
            eval_counter: 0,
            eval_period: eval_period.max(1),
            fade_samples: fade_samples.max(1),
            history: vec![0.0; history_len],
            history_write: 0,
            stutter_read: 0,
        }
    }

    fn pick_drop_duration_samples(&mut self) -> u32 {
        let min = self.params.min_drop_ms.max(MIN_DROP_MS).min(self.params.max_drop_ms.max(MIN_DROP_MS));
        let max = self.params.max_drop_ms.max(min);
        let span = max - min;
        let pick_ms = if span <= 0.0 { min } else { min + self.rng.f32() * span };
        ((pick_ms * 0.001) * self.sample_rate as f32) as u32
    }
}

impl Effect for PacketLoss {
    fn type_name(&self) -> &'static str { "packetLoss" }

    fn process(&mut self, buffer: &mut [f32]) {
        let drop_chance = self.params.drop_chance.clamp(0.0, MAX_DROP_CHANCE);
        let stutter_chance = self.params.stutter_chance.clamp(0.0, MAX_STUTTER_CHANCE);

        for s in buffer {
            // Store input into history before any modification.
            self.history[self.history_write] = *s;
            self.history_write = (self.history_write + 1) % self.history.len();

            // Periodically roll for new drop.
            self.eval_counter += 1;
            if self.eval_counter >= self.eval_period {
                self.eval_counter = 0;
                if matches!(self.state, State::PassThrough)
                    && drop_chance > 0.0
                    && self.rng.f32() < drop_chance
                {
                    let mode = if self.rng.f32() < stutter_chance {
                        // Stutter mode: replay history starting from oldest sample.
                        self.stutter_read = self.history_write; // oldest after wrap
                        DropMode::Stutter
                    } else {
                        DropMode::Silent
                    };
                    let _drop_dur = self.pick_drop_duration_samples();
                    self.state = State::FadingOut {
                        remaining: self.fade_samples,
                        total: self.fade_samples,
                        mode,
                    };
                }
            }

            *s = match self.state {
                State::PassThrough => *s,
                State::FadingOut { remaining, total, mode } => {
                    let mix = remaining as f32 / total as f32; // 1.0 → 0.0
                    let dry = *s;
                    let dropped = sample_drop(self, mode);
                    let out = dry * mix + dropped * (1.0 - mix);
                    let next = remaining.saturating_sub(1);
                    if next == 0 {
                        let drop_dur = self.pick_drop_duration_samples();
                        self.state = State::Dropped { remaining: drop_dur, mode };
                    } else {
                        self.state = State::FadingOut { remaining: next, total, mode };
                    }
                    out
                }
                State::Dropped { remaining, mode } => {
                    let out = sample_drop(self, mode);
                    let next = remaining.saturating_sub(1);
                    if next == 0 {
                        self.state = State::FadingIn {
                            remaining: self.fade_samples,
                            total: self.fade_samples,
                            mode,
                        };
                    } else {
                        self.state = State::Dropped { remaining: next, mode };
                    }
                    out
                }
                State::FadingIn { remaining, total, mode } => {
                    let mix = remaining as f32 / total as f32; // 1.0 → 0.0
                    let dry = *s;
                    let dropped = sample_drop(self, mode);
                    let out = dropped * mix + dry * (1.0 - mix);
                    let next = remaining.saturating_sub(1);
                    if next == 0 {
                        self.state = State::PassThrough;
                    } else {
                        self.state = State::FadingIn { remaining: next, total, mode };
                    }
                    out
                }
            };
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: PacketLossParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.drop_chance = p.drop_chance.clamp(0.0, MAX_DROP_CHANCE);
        p.min_drop_ms = p.min_drop_ms.clamp(MIN_DROP_MS, MAX_DROP_MS);
        p.max_drop_ms = p.max_drop_ms.clamp(p.min_drop_ms, MAX_DROP_MS);
        p.stutter_chance = p.stutter_chance.clamp(0.0, MAX_STUTTER_CHANCE);
        self.params = p;
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("packet_loss params serialize")
    }

    fn reset(&mut self) {
        self.state = State::PassThrough;
        self.eval_counter = 0;
        self.history.iter_mut().for_each(|x| *x = 0.0);
        self.history_write = 0;
        self.stutter_read = 0;
    }
}

fn sample_drop(p: &mut PacketLoss, mode: DropMode) -> f32 {
    match mode {
        DropMode::Silent => 0.0,
        DropMode::Stutter => {
            let v = p.history[p.stutter_read];
            p.stutter_read = (p.stutter_read + 1) % p.history.len();
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_chance_is_identity() {
        let mut pl = PacketLoss::new(48000, PacketLossParams::default());
        let mut buf = vec![0.1, 0.2, -0.3, 0.4];
        pl.process(&mut buf);
        let expected = [0.1, 0.2, -0.3, 0.4];
        for (a, e) in buf.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-5);
        }
    }

    #[test]
    fn high_chance_produces_silent_regions() {
        let mut pl = PacketLoss::new(48000, PacketLossParams {
            drop_chance: 0.5,
            min_drop_ms: 30.0,
            max_drop_ms: 30.0,
            stutter_chance: 0.0,
        });
        // Constant input — drops manifest as zeros.
        let mut buf = vec![0.5_f32; 48000];
        pl.process(&mut buf);
        let zero_count = buf.iter().filter(|s| s.abs() < 0.01).count();
        assert!(zero_count > 100, "expected silent regions, got {} zero samples", zero_count);
    }

    #[test]
    fn fade_ramps_avoid_discontinuity() {
        let mut pl = PacketLoss::new(48000, PacketLossParams {
            drop_chance: 0.5,
            min_drop_ms: 30.0,
            max_drop_ms: 30.0,
            stutter_chance: 0.0,
        });
        let mut buf = vec![0.5_f32; 48000];
        pl.process(&mut buf);
        // No pair of consecutive samples should differ by more than ~0.5 (dry signal
        // amplitude). Pre-ramp it's ~0.005 per sample over a 96-sample fade.
        for w in buf.windows(2) {
            let diff = (w[1] - w[0]).abs();
            assert!(diff < 0.1, "discontinuity {} between adjacent samples", diff);
        }
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut pl = PacketLoss::new(48000, PacketLossParams::default());
        pl.set_params(&serde_json::json!({
            "dropChance": 10.0,
            "minDropMs": 0.0,
            "maxDropMs": 99999.0,
            "stutterChance": 5.0
        })).unwrap();
        let p: PacketLossParams = serde_json::from_value(pl.get_params()).unwrap();
        assert_eq!(p.drop_chance, MAX_DROP_CHANCE);
        assert_eq!(p.min_drop_ms, MIN_DROP_MS);
        assert_eq!(p.max_drop_ms, MAX_DROP_MS);
        assert_eq!(p.stutter_chance, MAX_STUTTER_CHANCE);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::packet_loss`
Expected: 4 tests pass.

---

## Task 7: Bad noise gate

**Files:**
- Create: `E:\ClaudeCode\ratmic\src-tauri\src\effects\noise_gate.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs`

Envelope-based gate with attack/release. Chatter parameter introduces random false-closes when the input is just above threshold.

- [ ] **Step 1: Register module**

Edit `src-tauri/src/effects/mod.rs`, add `pub mod noise_gate;`.

- [ ] **Step 2: Implement NoiseGate with tests**

Create `src-tauri/src/effects/noise_gate.rs`:

```rust
//! Bad noise gate: envelope follower + threshold, plus random chatter near the threshold.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::Effect;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseGateParams {
    #[serde(rename = "thresholdDb", default = "default_threshold")]
    pub threshold_db: f32,
    #[serde(rename = "attackMs", default = "default_attack")]
    pub attack_ms: f32,
    #[serde(rename = "releaseMs", default = "default_release")]
    pub release_ms: f32,
    #[serde(rename = "chatterAmount", default)]
    pub chatter_amount: f32,
}

fn default_threshold() -> f32 { -40.0 }
fn default_attack() -> f32 { 5.0 }
fn default_release() -> f32 { 80.0 }

impl Default for NoiseGateParams {
    fn default() -> Self {
        Self {
            threshold_db: default_threshold(),
            attack_ms: default_attack(),
            release_ms: default_release(),
            chatter_amount: 0.0,
        }
    }
}

const MIN_THRESH_DB: f32 = -60.0;
const MAX_THRESH_DB: f32 = 0.0;
const MIN_TIME_MS: f32 = 0.5;
const MAX_ATTACK_MS: f32 = 200.0;
const MAX_RELEASE_MS: f32 = 500.0;
/// Window above threshold (in dB) where chatter is allowed to false-close.
const CHATTER_BAND_DB: f32 = 6.0;
/// Number of samples between chatter dice rolls.
const CHATTER_EVAL_SAMPLES: u32 = 480; // 10 ms @ 48 kHz

pub struct NoiseGate {
    params: NoiseGateParams,
    sample_rate: u32,
    rng: fastrand::Rng,
    envelope: f32,
    /// Current gate openness, 0.0..=1.0, smoothed via attack/release.
    open: f32,
    target_open: f32,
    attack_coef: f32,
    release_coef: f32,
    threshold_amp: f32,
    chatter_band_low: f32,
    /// Samples remaining in a chatter-forced close.
    chatter_close_remaining: u32,
    chatter_counter: u32,
}

impl NoiseGate {
    pub fn new(sample_rate: u32, params: NoiseGateParams) -> Self {
        let mut g = Self {
            params,
            sample_rate,
            rng: fastrand::Rng::new(),
            envelope: 0.0,
            open: 0.0,
            target_open: 0.0,
            attack_coef: 0.0,
            release_coef: 0.0,
            threshold_amp: 0.0,
            chatter_band_low: 0.0,
            chatter_close_remaining: 0,
            chatter_counter: 0,
        };
        g.recompute();
        g
    }

    fn recompute(&mut self) {
        let sr = self.sample_rate as f32;
        let attack = self.params.attack_ms.max(MIN_TIME_MS);
        let release = self.params.release_ms.max(MIN_TIME_MS);
        self.attack_coef = (-1.0 / (attack * 0.001 * sr)).exp();
        self.release_coef = (-1.0 / (release * 0.001 * sr)).exp();
        self.threshold_amp = 10.0_f32.powf(self.params.threshold_db / 20.0);
        self.chatter_band_low = 10.0_f32
            .powf((self.params.threshold_db + CHATTER_BAND_DB) / 20.0);
    }
}

impl Effect for NoiseGate {
    fn type_name(&self) -> &'static str { "noiseGate" }

    fn process(&mut self, buffer: &mut [f32]) {
        let chatter = self.params.chatter_amount.clamp(0.0, 1.0);
        let env_attack = (-1.0 / (0.001 * self.sample_rate as f32)).exp(); // 1 ms env attack
        let env_release = (-1.0 / (0.030 * self.sample_rate as f32)).exp(); // 30 ms env release

        for s in buffer {
            let abs = s.abs();
            let coef = if abs > self.envelope { env_attack } else { env_release };
            self.envelope = abs + (self.envelope - abs) * coef;

            // Threshold decision.
            self.target_open = if self.envelope >= self.threshold_amp { 1.0 } else { 0.0 };

            // Chatter: occasionally force-close if envelope is in the band just above threshold.
            if chatter > 0.0
                && self.envelope > self.threshold_amp
                && self.envelope < self.chatter_band_low
            {
                self.chatter_counter += 1;
                if self.chatter_counter >= CHATTER_EVAL_SAMPLES {
                    self.chatter_counter = 0;
                    if self.rng.f32() < chatter {
                        // Force closed for 5..=15 ms.
                        let ms = 5.0 + self.rng.f32() * 10.0;
                        self.chatter_close_remaining =
                            ((ms * 0.001) * self.sample_rate as f32) as u32;
                    }
                }
            } else {
                self.chatter_counter = 0;
            }
            if self.chatter_close_remaining > 0 {
                self.chatter_close_remaining -= 1;
                self.target_open = 0.0;
            }

            // Smooth toward target via attack/release.
            let coef = if self.target_open > self.open { self.attack_coef } else { self.release_coef };
            self.open = self.target_open + (self.open - self.target_open) * coef;

            *s *= self.open;
        }
    }

    fn set_params(&mut self, params: &Json) -> Result<()> {
        let mut p: NoiseGateParams = serde_json::from_value(params.clone()).unwrap_or_default();
        p.threshold_db = p.threshold_db.clamp(MIN_THRESH_DB, MAX_THRESH_DB);
        p.attack_ms = p.attack_ms.clamp(MIN_TIME_MS, MAX_ATTACK_MS);
        p.release_ms = p.release_ms.clamp(MIN_TIME_MS, MAX_RELEASE_MS);
        p.chatter_amount = p.chatter_amount.clamp(0.0, 1.0);
        self.params = p;
        self.recompute();
        Ok(())
    }

    fn get_params(&self) -> Json {
        serde_json::to_value(&self.params).expect("noise_gate params serialize")
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
        self.open = 0.0;
        self.target_open = 0.0;
        self.chatter_close_remaining = 0;
        self.chatter_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_signal_is_gated_to_silence() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            chatter_amount: 0.0,
        });
        let mut buf = vec![0.01_f32; 4800]; // -40 dB
        g.process(&mut buf);
        let tail_max = buf[3000..].iter().cloned().fold(0.0_f32, |a, b| a.max(b.abs()));
        assert!(tail_max < 0.001, "quiet signal should be gated, got {}", tail_max);
    }

    #[test]
    fn loud_signal_passes_through() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            chatter_amount: 0.0,
        });
        let mut buf = vec![0.5_f32; 4800]; // -6 dB
        g.process(&mut buf);
        let tail_avg: f32 = buf[3000..].iter().sum::<f32>() / buf[3000..].len() as f32;
        assert!(tail_avg > 0.4, "loud signal should pass, got avg {}", tail_avg);
    }

    #[test]
    fn release_avoids_immediate_click_on_quiet() {
        let mut g = NoiseGate::new(48000, NoiseGateParams {
            threshold_db: -30.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            chatter_amount: 0.0,
        });
        // First 1000 samples loud, next 4000 quiet.
        let mut buf = Vec::with_capacity(5000);
        for _ in 0..1000 { buf.push(0.5_f32); }
        for _ in 0..4000 { buf.push(0.0_f32); }
        g.process(&mut buf);
        // The transition at sample 1000 should not produce a sample-to-sample
        // discontinuity larger than ~0.05.
        for w in buf.windows(2).skip(990).take(20) {
            let diff = (w[1] - w[0]).abs();
            assert!(diff < 0.05, "discontinuity {} at transition", diff);
        }
    }

    #[test]
    fn params_clamp_to_safe_range() {
        let mut g = NoiseGate::new(48000, NoiseGateParams::default());
        g.set_params(&serde_json::json!({
            "thresholdDb": 100.0,
            "attackMs": 0.0,
            "releaseMs": 9999.0,
            "chatterAmount": 2.0
        })).unwrap();
        let p: NoiseGateParams = serde_json::from_value(g.get_params()).unwrap();
        assert_eq!(p.threshold_db, MAX_THRESH_DB);
        assert_eq!(p.attack_ms, MIN_TIME_MS);
        assert_eq!(p.release_ms, MAX_RELEASE_MS);
        assert_eq!(p.chatter_amount, 1.0);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test effects::noise_gate`
Expected: 4 tests pass.

---

## Task 8: Extend default chain in audio engine

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\engine.rs`
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs` (verify all modules registered)

Build the default chain in a sensible order: Gain → Bandpass → Bitcrusher → Clipper → Noise → PacketLoss → NoiseGate → Limiter. All disabled by default except the fixed final Limiter.

- [ ] **Step 1: Verify `effects/mod.rs` registers all modules**

Open `src-tauri/src/effects/mod.rs` and confirm it contains (order doesn't matter, but all must be present):

```rust
pub mod bandpass;
pub mod biquad;
pub mod bitcrusher;
pub mod chain;
pub mod clipper;
pub mod crossfade;
pub mod gain;
pub mod limiter;
pub mod noise;
pub mod noise_gate;
pub mod packet_loss;
```

Keep the existing `Effect` trait definition intact at the bottom of the file.

- [ ] **Step 2: Update `AudioEngine::start` to build the full default chain**

Open `src-tauri/src/audio/engine.rs` and replace the chain construction block. Find the section that currently reads:

```rust
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
```

Replace it with:

```rust
        // Build default chain: 8 slots in fixed order, all disabled except the
        // fixed final Limiter. Reordering and add/remove come in the presets plan.
        let mut chain = EffectChain::new(INTERNAL_SAMPLE_RATE);
        chain.push(
            Box::new(Gain::new(GainParams::default())),
            false,
        );
        chain.push(
            Box::new(Bandpass::new(INTERNAL_SAMPLE_RATE, BandpassParams::default())),
            false,
        );
        chain.push(
            Box::new(Bitcrusher::new(INTERNAL_SAMPLE_RATE, BitcrusherParams::default())),
            false,
        );
        chain.push(
            Box::new(Clipper::new(ClipperParams::default())),
            false,
        );
        chain.push(
            Box::new(Noise::new(INTERNAL_SAMPLE_RATE, NoiseParams::default())),
            false,
        );
        chain.push(
            Box::new(PacketLoss::new(INTERNAL_SAMPLE_RATE, PacketLossParams::default())),
            false,
        );
        chain.push(
            Box::new(NoiseGate::new(INTERNAL_SAMPLE_RATE, NoiseGateParams::default())),
            false,
        );
        chain.push(
            Box::new(Limiter::new(INTERNAL_SAMPLE_RATE, LimiterParams::default())),
            true,
        );
        let chain = Arc::new(Mutex::new(chain));
```

- [ ] **Step 3: Update imports at the top of `engine.rs`**

The existing file has three `use crate::effects::...` lines near the top (chain, gain, limiter). Replace those three lines with this expanded set so all eight effect types are in scope:

```rust
use crate::effects::bandpass::{Bandpass, BandpassParams};
use crate::effects::bitcrusher::{Bitcrusher, BitcrusherParams};
use crate::effects::chain::EffectChain;
use crate::effects::clipper::{Clipper, ClipperParams};
use crate::effects::gain::{Gain, GainParams};
use crate::effects::limiter::{Limiter, LimiterParams};
use crate::effects::noise::{Noise, NoiseParams};
use crate::effects::noise_gate::{NoiseGate, NoiseGateParams};
use crate::effects::packet_loss::{PacketLoss, PacketLossParams};
```

- [ ] **Step 4: Update the static default in `commands.rs::get_chain`**

Open `src-tauri/src/commands.rs`. Find the `get_chain` command and locate the static-default fallback (used when engine is not running). Replace the hardcoded 2-slot default with all 8 slots. The relevant block currently looks like:

```rust
    let Some(engine) = guard.as_ref() else {
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
```

Replace with:

```rust
    let Some(engine) = guard.as_ref() else {
        use crate::effects::bandpass::BandpassParams;
        use crate::effects::bitcrusher::BitcrusherParams;
        use crate::effects::clipper::ClipperParams;
        use crate::effects::noise::NoiseParams;
        use crate::effects::noise_gate::NoiseGateParams;
        use crate::effects::packet_loss::PacketLossParams;
        return vec![
            ChainSlotView { index: 0, type_name: "gain".into(),       enabled: false, params: serde_json::to_value(GainParams::default()).unwrap() },
            ChainSlotView { index: 1, type_name: "bandpass".into(),   enabled: false, params: serde_json::to_value(BandpassParams::default()).unwrap() },
            ChainSlotView { index: 2, type_name: "bitcrusher".into(), enabled: false, params: serde_json::to_value(BitcrusherParams::default()).unwrap() },
            ChainSlotView { index: 3, type_name: "clipper".into(),    enabled: false, params: serde_json::to_value(ClipperParams::default()).unwrap() },
            ChainSlotView { index: 4, type_name: "noise".into(),      enabled: false, params: serde_json::to_value(NoiseParams::default()).unwrap() },
            ChainSlotView { index: 5, type_name: "packetLoss".into(), enabled: false, params: serde_json::to_value(PacketLossParams::default()).unwrap() },
            ChainSlotView { index: 6, type_name: "noiseGate".into(),  enabled: false, params: serde_json::to_value(NoiseGateParams::default()).unwrap() },
            ChainSlotView { index: 7, type_name: "limiter".into(),    enabled: true,  params: serde_json::json!({ "ceilingDb": -3.0, "releaseMs": 80.0 }) },
        ];
    };
```

- [ ] **Step 5: Verify everything compiles and all tests pass**

Run: `cd src-tauri && cargo build`
Expected: clean build (dead-code warnings on yet-unused trait helpers acceptable).

Run: `cd src-tauri && cargo test`
Expected: prior 37 tests + new effect tests (7 biquad + 6 clipper + 5 bitcrusher + 5 bandpass + 5 noise + 4 packet_loss + 4 noise_gate = 36 new) = **73 tests pass**.

---

## Task 9: Per-effect editor components + dispatcher refactor

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\GainEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\BandpassEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\BitcrusherEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\ClipperEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\NoiseEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\PacketLossEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\NoiseGateEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\LimiterEditor.svelte`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\effects\Slider.svelte` (shared)
- Modify: `E:\ClaudeCode\ratmic\src\lib\components\EffectParams.svelte` (refactored dispatcher)

Each editor receives the current slot, owns the parameter-update flow for its effect type. `Slider.svelte` is a tiny shared component to keep each editor focused.

- [ ] **Step 1: Create shared Slider component**

Create `src/lib/components/effects/Slider.svelte`:

```svelte
<script lang="ts">
  export let label: string;
  export let value: number;
  export let min: number;
  export let max: number;
  export let step: number = 0.1;
  export let unit: string = "";
  export let onChange: (v: number) => void;
</script>

<label class="slider">
  <span class="row">
    <span class="label">{label}</span>
    <span class="value">{value.toFixed(step >= 1 ? 0 : 2)}{unit}</span>
  </span>
  <input
    type="range"
    {min}
    {max}
    {step}
    {value}
    on:input={(e) => onChange(parseFloat((e.target as HTMLInputElement).value))}
  />
</label>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    margin-bottom: 0.75rem;
    font-size: 12px;
    color: var(--text-1);
  }
  .row {
    display: flex;
    justify-content: space-between;
  }
  .value {
    color: var(--text-0);
    font-variant-numeric: tabular-nums;
  }
  input[type="range"] {
    width: 100%;
  }
</style>
```

- [ ] **Step 2: Create GainEditor**

Create `src/lib/components/effects/GainEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { gainDb?: number }) ?? {};

  async function setGainDb(value: number) {
    const next = { ...params, gainDb: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider
  label="Gain"
  value={params.gainDb ?? 0}
  min={-24}
  max={24}
  step={0.5}
  unit=" dB"
  onChange={setGainDb}
/>
```

- [ ] **Step 3: Create BandpassEditor**

Create `src/lib/components/effects/BandpassEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { lowCutHz?: number; highCutHz?: number; midBoostDb?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Low cut"  value={params.lowCutHz ?? 100}  min={20}    max={8000}  step={10}  unit=" Hz" onChange={(v) => setParam("lowCutHz", v)} />
<Slider label="High cut" value={params.highCutHz ?? 8000} min={200}  max={20000} step={50}  unit=" Hz" onChange={(v) => setParam("highCutHz", v)} />
<Slider label="Mid boost" value={params.midBoostDb ?? 0} min={-12}  max={12}    step={0.5} unit=" dB" onChange={(v) => setParam("midBoostDb", v)} />
```

- [ ] **Step 4: Create BitcrusherEditor**

Create `src/lib/components/effects/BitcrusherEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { bitDepth?: number; sampleRateHz?: number; mix?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Bit depth"   value={params.bitDepth ?? 16}      min={1}   max={16}    step={1}    unit=" bit"  onChange={(v) => setParam("bitDepth", v)} />
<Slider label="Sample rate" value={params.sampleRateHz ?? 48000} min={1000} max={48000} step={100}  unit=" Hz"  onChange={(v) => setParam("sampleRateHz", v)} />
<Slider label="Mix"         value={params.mix ?? 1.0}           min={0}   max={1}     step={0.05} onChange={(v) => setParam("mix", v)} />
```

- [ ] **Step 5: Create ClipperEditor**

Create `src/lib/components/effects/ClipperEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { drive?: number; hardClip?: number; softClip?: number; outputTrimDb?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Drive"     value={params.drive ?? 1.0}         min={1}   max={10}  step={0.1}  onChange={(v) => setParam("drive", v)} />
<Slider label="Hard clip" value={params.hardClip ?? 1.0}      min={0.1} max={1}   step={0.05} onChange={(v) => setParam("hardClip", v)} />
<Slider label="Soft clip" value={params.softClip ?? 0.0}      min={0}   max={1}   step={0.05} onChange={(v) => setParam("softClip", v)} />
<Slider label="Trim"      value={params.outputTrimDb ?? 0}    min={-24} max={6}   step={0.5}  unit=" dB" onChange={(v) => setParam("outputTrimDb", v)} />
```

- [ ] **Step 6: Create NoiseEditor**

Create `src/lib/components/effects/NoiseEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as {
    whiteAmount?: number;
    humAmount?: number;
    humHz?: number;
    crackleRate?: number;
    gateMode?: "always" | "onspeech";
    speechThresholdDb?: number;
  }) ?? {};

  async function setParam(key: string, value: number | string) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="White"   value={params.whiteAmount ?? 0}    min={0}  max={0.5} step={0.01} onChange={(v) => setParam("whiteAmount", v)} />
<Slider label="Hum"     value={params.humAmount ?? 0}      min={0}  max={0.5} step={0.01} onChange={(v) => setParam("humAmount", v)} />
<Slider label="Hum Hz"  value={params.humHz ?? 60}         min={40} max={120} step={1} unit=" Hz" onChange={(v) => setParam("humHz", v)} />
<Slider label="Crackle" value={params.crackleRate ?? 0}    min={0}  max={50}  step={0.5} unit="/s" onChange={(v) => setParam("crackleRate", v)} />

<label class="row">
  <span>Gate</span>
  <select
    value={params.gateMode ?? "always"}
    on:change={(e) => setParam("gateMode", (e.target as HTMLSelectElement).value)}
  >
    <option value="always">always</option>
    <option value="onspeech">on speech</option>
  </select>
</label>

{#if params.gateMode === "onspeech"}
  <Slider label="Speech thresh" value={params.speechThresholdDb ?? -40} min={-90} max={0} step={1} unit=" dB" onChange={(v) => setParam("speechThresholdDb", v)} />
{/if}

<style>
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    font-size: 12px;
    color: var(--text-1);
    margin-bottom: 0.75rem;
  }
</style>
```

- [ ] **Step 7: Create PacketLossEditor**

Create `src/lib/components/effects/PacketLossEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { dropChance?: number; minDropMs?: number; maxDropMs?: number; stutterChance?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Drop chance"    value={params.dropChance ?? 0}     min={0}  max={0.5} step={0.01} onChange={(v) => setParam("dropChance", v)} />
<Slider label="Min drop"       value={params.minDropMs ?? 30}     min={10} max={500} step={5}  unit=" ms" onChange={(v) => setParam("minDropMs", v)} />
<Slider label="Max drop"       value={params.maxDropMs ?? 140}    min={10} max={500} step={5}  unit=" ms" onChange={(v) => setParam("maxDropMs", v)} />
<Slider label="Stutter chance" value={params.stutterChance ?? 0}  min={0}  max={0.3} step={0.01} onChange={(v) => setParam("stutterChance", v)} />
```

- [ ] **Step 8: Create NoiseGateEditor**

Create `src/lib/components/effects/NoiseGateEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { thresholdDb?: number; attackMs?: number; releaseMs?: number; chatterAmount?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Threshold" value={params.thresholdDb ?? -40} min={-60} max={0}   step={1}    unit=" dB" onChange={(v) => setParam("thresholdDb", v)} />
<Slider label="Attack"    value={params.attackMs ?? 5}      min={0.5} max={200} step={0.5}  unit=" ms" onChange={(v) => setParam("attackMs", v)} />
<Slider label="Release"   value={params.releaseMs ?? 80}    min={0.5} max={500} step={1}    unit=" ms" onChange={(v) => setParam("releaseMs", v)} />
<Slider label="Chatter"   value={params.chatterAmount ?? 0} min={0}   max={1}   step={0.05} onChange={(v) => setParam("chatterAmount", v)} />
```

- [ ] **Step 9: Create LimiterEditor**

Create `src/lib/components/effects/LimiterEditor.svelte`:

```svelte
<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { ceilingDb?: number; releaseMs?: number }) ?? {};

  async function setParam(key: string, value: number) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<Slider label="Ceiling" value={params.ceilingDb ?? -3} min={-24} max={0}   step={0.5} unit=" dB" onChange={(v) => setParam("ceilingDb", v)} />
<Slider label="Release" value={params.releaseMs ?? 80} min={1}   max={500} step={1}   unit=" ms" onChange={(v) => setParam("releaseMs", v)} />
```

- [ ] **Step 10: Refactor EffectParams.svelte into a dispatcher**

Replace `src/lib/components/EffectParams.svelte` entirely:

```svelte
<script lang="ts">
  import { chain, selectedEffectIndex } from "../stores";
  import GainEditor from "./effects/GainEditor.svelte";
  import BandpassEditor from "./effects/BandpassEditor.svelte";
  import BitcrusherEditor from "./effects/BitcrusherEditor.svelte";
  import ClipperEditor from "./effects/ClipperEditor.svelte";
  import NoiseEditor from "./effects/NoiseEditor.svelte";
  import PacketLossEditor from "./effects/PacketLossEditor.svelte";
  import NoiseGateEditor from "./effects/NoiseGateEditor.svelte";
  import LimiterEditor from "./effects/LimiterEditor.svelte";

  $: slot = $selectedEffectIndex !== null
    ? $chain.find((s) => s.index === $selectedEffectIndex)
    : null;
</script>

<h3>Parameters</h3>

{#if !slot}
  <p class="muted">Select an effect to edit its parameters.</p>
{:else if slot.type_name === "gain"}
  <GainEditor {slot} />
{:else if slot.type_name === "bandpass"}
  <BandpassEditor {slot} />
{:else if slot.type_name === "bitcrusher"}
  <BitcrusherEditor {slot} />
{:else if slot.type_name === "clipper"}
  <ClipperEditor {slot} />
{:else if slot.type_name === "noise"}
  <NoiseEditor {slot} />
{:else if slot.type_name === "packetLoss"}
  <PacketLossEditor {slot} />
{:else if slot.type_name === "noiseGate"}
  <NoiseGateEditor {slot} />
{:else if slot.type_name === "limiter"}
  <LimiterEditor {slot} />
{:else}
  <p class="muted">No editor for "{slot.type_name}" yet.</p>
{/if}

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .muted { color: var(--text-2); font-size: 12px; }
</style>
```

- [ ] **Step 11: Verify TypeScript checks pass**

Run: `npm run check`
Expected: 0 errors.

---

## Task 10: Phase smoke test (manual, human-verified)

After the implementer dispatch reports DONE, the user must run the app and verify the new effects work audibly.

- [ ] **Step 1: Launch the app**

Run: `npm run tauri dev`
Expected: window opens, effect chain shows 8 slots (gain, bandpass, bitcrusher, clipper, noise, packetLoss, noiseGate, limiter — limiter has the "fixed" badge).

- [ ] **Step 2: Verify each effect's editor renders**

Click each row in turn. The right pane should show the appropriate sliders for each effect. No console errors.

- [ ] **Step 3: Audible tests (one at a time, with engine started)**

Pick an input and output device, click START.

For each test, enable ONLY the named effect (plus Gain at default 0 dB and the always-on Limiter); leave the others disabled. Speak into the mic and listen for the expected character.

| Effect | Setting | Expected sound |
|---|---|---|
| Bandpass | low=300, high=3400, midBoost=0 | Telephone-y, no lows or highs |
| Bandpass | low=300, high=3400, midBoost=+9 dB | Nasal/honky telephone |
| Bitcrusher | bits=8, sr=11025, mix=0.8 | Crunchy, sample-rate-reduced |
| Bitcrusher | bits=1, sr=8000, mix=1.0 | Square-wave-y, very distorted |
| Clipper | drive=4, hard=0.5, soft=0.2 | Distorted, heavier with louder input |
| Noise | white=0.1, gate=always | Constant hiss layered on voice |
| Noise | white=0.2, gate=onspeech, threshold=-40 | Hiss only while speaking |
| Noise | hum=0.15, hz=60 | Low-frequency hum buzz |
| Noise | crackle=10 | Random click-pops |
| Packet Loss | drop=0.15, min=30, max=140, stutter=0 | Random silent dropouts, no clicks |
| Packet Loss | drop=0.15, stutter=0.2 | Some dropouts replay recent audio |
| Noise Gate | threshold=-30, chatter=0 | Cuts off below threshold |
| Noise Gate | threshold=-30, chatter=0.8 | Stutters near threshold (bad gate effect) |
| Limiter | ceiling=-12 (with Gain at +12) | Output clamped harder than default |

- [ ] **Step 4: Combine — full chain test**

Enable all 8 effects with their defaults. Tweak Gain to +6 dB. Speak. Output should be a layered "bad mic" character, but the Limiter must keep output ≤ −3 dB peak (verify in the output meter). No painful spikes when toggling on/off.

- [ ] **Step 5: Verify no regressions**

Run: `cd src-tauri && cargo test`
Expected: all 73 tests pass.

Run: `npm run check`
Expected: 0 errors.

---

## Final verification

When all 10 tasks are checked off:

- [ ] 73 Rust tests pass.
- [ ] `npm run check` clean.
- [ ] App launches, 8 effect rows visible.
- [ ] Each effect changes audio character when enabled.
- [ ] Toggling effects on/off produces no clicks (Phase 2 crossfade still works).
- [ ] Limiter still prevents output from exceeding ceiling, even with all effects enabled and Gain pushed high.

Phase 3 (hotkey + monitor + test record), Phase 4 (presets), and Phase 5 (routing health + Safe Mode + resampling) will be planned in separate documents once this effects phase is verified working.
