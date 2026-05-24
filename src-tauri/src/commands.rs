use parking_lot::Mutex;
use serde_json::Value as Json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

use crate::audio::devices::{list_devices, DeviceInfo};
use crate::audio::engine::{AudioEngine, MeterSink, MeterSnapshot};
use crate::events::{EngineStateEvent, MeterEvent, EVENT_ENGINE_STATE, EVENT_METERS};
use crate::settings::HotkeyConfig;
use crate::presets::builtin;
use crate::presets::schema::{EffectInstance, Preset};
use crate::presets::user;
use crate::settings::Settings;

pub struct AppState {
    pub engine: Mutex<Option<AudioEngine>>,
    pub effects_enabled: Arc<AtomicBool>,
    pub hotkey: Mutex<Option<crate::hotkeys::HotkeyHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
            effects_enabled: Arc::new(AtomicBool::new(true)),
            hotkey: Mutex::new(None),
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
            limiter_activity_pct: snap.limiter_activity_pct,
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
    monitor_id: Option<String>,
    monitor_enabled: bool,
) -> Result<(), String> {
    let mut guard = state.engine.lock();
    if guard.is_some() {
        return Err("engine already running".into());
    }
    let sink = EmitSink { app: app.clone() };
    let engine = AudioEngine::start(
        &input_id,
        &output_id,
        monitor_id.as_deref(),
        monitor_enabled,
        state.effects_enabled.clone(),
        sink,
    )
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
pub fn set_monitor_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    engine.set_monitor_enabled(enabled);
    Ok(())
}

#[tauri::command]
pub fn set_monitor_device(
    state: State<'_, AppState>,
    monitor_id: Option<String>,
) -> Result<(), String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    engine
        .set_monitor_device(monitor_id.as_deref())
        .map_err(|e| e.to_string())
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
        // Engine not running: return the default empty chain (just Limiter).
        return vec![
            ChainSlotView { index: 0, type_name: "limiter".into(), enabled: true, params: serde_json::json!({ "ceilingDb": -3.0, "releaseMs": 80.0 }) },
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
    let result = engine.chain.lock()
        .set_params(index, &params)
        .map_err(|e| e.to_string());
    result
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PresetSummary {
    pub name: String,
    pub description: Option<String>,
    pub builtin: bool,
    pub effect_types: Vec<String>,
}

#[tauri::command]
pub fn list_presets() -> Result<Vec<PresetSummary>, String> {
    fn types(p: &crate::presets::schema::Preset) -> Vec<String> {
        p.effects.iter().filter(|e| e.enabled).map(|e| e.type_.clone()).collect()
    }
    let mut out: Vec<PresetSummary> = builtin::all()
        .into_iter()
        .map(|p| PresetSummary {
            effect_types: types(&p),
            name: p.name,
            description: p.description,
            builtin: true,
        })
        .collect();
    let users = user::list().map_err(|e| e.to_string())?;
    for p in users {
        out.push(PresetSummary {
            effect_types: types(&p),
            name: p.name,
            description: p.description,
            builtin: false,
        });
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
    drop(guard);

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

#[tauri::command]
pub fn reorder_effects(
    state: State<'_, AppState>,
    from: usize,
    to: usize,
) -> Result<bool, String> {
    let guard = state.engine.lock();
    let Some(engine) = guard.as_ref() else {
        return Err("engine not running".into());
    };
    Ok(engine.reorder_effects(from, to))
}

#[tauri::command]
pub fn set_effects_enabled(state: State<'_, AppState>, enabled: bool) {
    state.effects_enabled.store(enabled, Ordering::Relaxed);
}

#[tauri::command]
pub fn effects_enabled(state: State<'_, AppState>) -> bool {
    state.effects_enabled.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_hotkey(
    app: AppHandle,
    state: State<'_, AppState>,
    config: HotkeyConfig,
) -> Result<(), String> {
    // Replace any existing registration.
    let mut guard = state.hotkey.lock();
    if let Some(existing) = guard.take() {
        existing.unregister();
    }
    let mgr = crate::hotkeys::HotkeyHandle::register(
        &config,
        state.effects_enabled.clone(),
        app.clone(),
    )
    .map_err(|e| e.to_string())?;
    *guard = Some(mgr);
    drop(guard);

    // Persist.
    let mut s = Settings::load().map_err(|e| e.to_string())?;
    s.hotkey = Some(config);
    s.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clear_hotkey(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.hotkey.lock();
    if let Some(existing) = guard.take() {
        existing.unregister();
    }
    drop(guard);
    // Re-enable effects so the app isn't stuck bypassed.
    state.effects_enabled.store(true, Ordering::Relaxed);
    let mut s = Settings::load().map_err(|e| e.to_string())?;
    s.hotkey = None;
    s.save().map_err(|e| e.to_string())?;
    Ok(())
}

