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
