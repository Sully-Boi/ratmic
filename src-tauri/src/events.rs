use serde::Serialize;

pub const EVENT_METERS: &str = "meters";
pub const EVENT_ENGINE_STATE: &str = "engine-state";
pub const EVENT_EFFECTS_STATE: &str = "effects-state";

#[derive(Debug, Clone, Serialize)]
pub struct MeterEvent {
    pub input_peak_db: f32,
    pub input_rms_db: f32,
    pub output_peak_db: f32,
    pub output_rms_db: f32,
    pub limiter_activity_pct: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStateEvent {
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectsStateEvent {
    pub enabled: bool,
}
