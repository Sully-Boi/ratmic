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
