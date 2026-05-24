mod audio;
mod commands;
mod effects;
mod events;
mod hotkeys;
mod presets;
mod settings;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::new())
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
            commands::reorder_effects,
            commands::set_monitor_enabled,
            commands::set_monitor_device,
            commands::set_effects_enabled,
            commands::effects_enabled,
            commands::set_hotkey,
            commands::clear_hotkey,
        ])
        .setup(|app| {
            use tauri::Manager;
            let state = app.state::<AppState>();
            if let Ok(settings) = crate::settings::Settings::load() {
                if let Some(cfg) = settings.hotkey {
                    match crate::hotkeys::HotkeyHandle::register(
                        &cfg,
                        state.effects_enabled.clone(),
                        app.handle().clone(),
                    ) {
                        Ok(mgr) => *state.hotkey.lock() = Some(mgr),
                        Err(e) => log::warn!("could not register saved hotkey: {e}"),
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
