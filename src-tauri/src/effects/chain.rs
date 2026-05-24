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

    /// Returns true if the chain's Limiter was actively reducing gain in the
    /// most recent process() call. Returns false if no Limiter slot is present.
    pub fn limiter_was_active(&self) -> bool {
        self.slots
            .iter()
            .filter_map(|s| s.effect.limiter_was_active())
            .next()
            .unwrap_or(false)
    }

    /// Move the slot at `from` to index `to`, preserving effect instances.
    /// `to` is interpreted as the destination index AFTER the element is removed.
    /// Refuses (returns false) if either index is out of range, or if `from`/`to`
    /// would move or displace the fixed Limiter slot.
    pub fn move_slot(&mut self, from: usize, to: usize) -> bool {
        let n = self.slots.len();
        if from >= n || to >= n {
            return false;
        }
        let limiter_idx = self
            .slots
            .iter()
            .rposition(|s| s.effect.type_name() == "limiter");
        if let Some(li) = limiter_idx {
            // Cannot move the limiter itself, nor place anything at/after it.
            if from == li || to >= li {
                return false;
            }
        }
        if from == to {
            return true;
        }
        let slot = self.slots.remove(from);
        self.slots.insert(to, slot);
        true
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

    #[test]
    fn rebuild_appends_limiter_automatically() {
        use crate::effects::gain::{Gain, GainParams};
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

    #[test]
    fn move_slot_reorders_preserving_limiter_last() {
        use crate::effects::gain::{Gain, GainParams};
        use crate::effects::clipper::{Clipper, ClipperParams};
        use crate::effects::bitcrusher::{Bitcrusher, BitcrusherParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![
            (Box::new(Gain::new(GainParams::default())), true),
            (Box::new(Clipper::new(ClipperParams::default())), true),
            (Box::new(Bitcrusher::new(48000, BitcrusherParams::default())), true),
        ]);
        // chain: [gain, clipper, bitcrusher, limiter]
        // move gain (0) to index 2 → [clipper, bitcrusher, gain, limiter]
        assert!(c.move_slot(0, 2));
        let view = c.slots_view();
        let types: Vec<&str> = view.iter().map(|s| s.0).collect();
        assert_eq!(types, vec!["clipper", "bitcrusher", "gain", "limiter"]);
    }

    #[test]
    fn move_slot_refuses_to_move_limiter() {
        use crate::effects::gain::{Gain, GainParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![(Box::new(Gain::new(GainParams::default())), true)]);
        // chain: [gain, limiter]; limiter is index 1
        assert!(!c.move_slot(1, 0), "must refuse to move the limiter");
        let types: Vec<&str> = c.slots_view().iter().map(|s| s.0).collect();
        assert_eq!(types, vec!["gain", "limiter"]);
    }

    #[test]
    fn move_slot_refuses_target_at_or_after_limiter() {
        use crate::effects::gain::{Gain, GainParams};
        use crate::effects::clipper::{Clipper, ClipperParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![
            (Box::new(Gain::new(GainParams::default())), true),
            (Box::new(Clipper::new(ClipperParams::default())), true),
        ]);
        // chain: [gain, clipper, limiter]; limiter index 2
        assert!(!c.move_slot(0, 2), "must refuse to displace the limiter");
        let types: Vec<&str> = c.slots_view().iter().map(|s| s.0).collect();
        assert_eq!(types, vec!["gain", "clipper", "limiter"]);
    }

    #[test]
    fn move_slot_out_of_range_is_false() {
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![]);
        assert!(!c.move_slot(0, 5));
        assert!(!c.move_slot(5, 0));
    }

    #[test]
    fn move_slot_same_index_is_noop_true() {
        use crate::effects::gain::{Gain, GainParams};
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![(Box::new(Gain::new(GainParams::default())), true)]);
        assert!(c.move_slot(0, 0));
    }

    #[test]
    fn chain_reports_limiter_activity() {
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![]);
        // Quiet input: no activity.
        let mut buf = vec![0.1; 256];
        c.process(&mut buf);
        assert!(!c.limiter_was_active());
        // Loud input: limiter kicks in.
        let mut buf = vec![0.95; 256];
        c.process(&mut buf);
        assert!(c.limiter_was_active());
    }
}
