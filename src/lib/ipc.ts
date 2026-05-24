import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type DeviceKind = "Input" | "Output";

export interface DeviceInfo {
  name: string;
  kind: DeviceKind;
  id: string;
  is_default: boolean;
}

export interface Settings {
  schema_version: number;
  input_device_id: string | null;
  output_device_id: string | null;
  monitor_enabled: boolean;
  monitor_device_id: string | null;
  safe_output_mode: boolean;
  last_preset_name: string | null;
  onboarding_seen: boolean;
}

export interface PresetSummary {
  name: string;
  description: string | null;
  builtin: boolean;
}

export interface MeterEvent {
  input_peak_db: number;
  input_rms_db: number;
  output_peak_db: number;
  output_rms_db: number;
  limiter_activity_pct: number;
}

export interface EngineStateEvent {
  running: boolean;
  error: string | null;
}

export interface ChainSlotView {
  index: number;
  type_name: string;
  enabled: boolean;
  params: unknown;
}

export const ipc = {
  loadSettings: () => invoke<Settings>("load_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listDevices: () => invoke<DeviceInfo[]>("list_audio_devices"),
  startEngine: (inputId: string, outputId: string, monitorId: string | null, monitorEnabled: boolean) =>
    invoke<void>("start_engine", { inputId, outputId, monitorId, monitorEnabled }),
  stopEngine: () => invoke<void>("stop_engine"),
  engineRunning: () => invoke<boolean>("engine_running"),
  getChain: () => invoke<ChainSlotView[]>("get_chain"),
  setEffectEnabled: (index: number, enabled: boolean) =>
    invoke<void>("set_effect_enabled", { index, enabled }),
  setEffectParams: (index: number, params: unknown) =>
    invoke<void>("set_effect_params", { index, params }),
  listPresets: () => invoke<PresetSummary[]>("list_presets"),
  loadPreset: (name: string, builtinPref: boolean) =>
    invoke<void>("load_preset", { name, builtinPref }),
  savePresetFromChain: (name: string, description: string | null) =>
    invoke<void>("save_preset_from_chain", { name, description }),
  deleteUserPreset: (name: string) =>
    invoke<void>("delete_user_preset", { name }),
  addEffect: (typeName: string) =>
    invoke<void>("add_effect", { typeName }),
  removeEffect: (index: number) =>
    invoke<boolean>("remove_effect", { index }),
  setMonitorEnabled: (enabled: boolean) =>
    invoke<void>("set_monitor_enabled", { enabled }),
};

export const events = {
  onMeters: (cb: (e: MeterEvent) => void): Promise<UnlistenFn> =>
    listen<MeterEvent>("meters", (e) => cb(e.payload)),
  onEngineState: (cb: (e: EngineStateEvent) => void): Promise<UnlistenFn> =>
    listen<EngineStateEvent>("engine-state", (e) => cb(e.payload)),
};
