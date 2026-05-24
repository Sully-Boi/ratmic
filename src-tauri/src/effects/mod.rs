//! Effect trait and module roots.

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
pub mod registry;

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

    /// For the Limiter, returns whether the limiter was actively reducing gain
    /// in the most recent process() call. Returns `None` for non-limiter effects.
    fn limiter_was_active(&self) -> Option<bool> {
        None
    }
}
