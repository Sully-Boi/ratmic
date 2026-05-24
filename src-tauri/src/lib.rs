mod audio;
mod commands;
mod effects;
mod events;
mod presets;
mod settings;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            commands::get_chain,
            commands::set_effect_enabled,
            commands::set_effect_params,
            commands::list_presets,
            commands::load_preset,
            commands::save_preset_from_chain,
            commands::delete_user_preset,
            commands::add_effect,
            commands::remove_effect,
            commands::set_monitor_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
