import { writable } from "svelte/store";
import type { Settings, MeterEvent, ChainSlotView, PresetSummary } from "./ipc";

export const settings = writable<Settings | null>(null);
export const inputDeviceId = writable<string | null>(null);
export const outputDeviceId = writable<string | null>(null);
export const engineRunning = writable<boolean>(false);
export const meters = writable<MeterEvent>({
  input_peak_db: -90,
  input_rms_db: -90,
  output_peak_db: -90,
  output_rms_db: -90,
  limiter_activity_pct: 0,
});
export const engineError = writable<string | null>(null);
export const chain = writable<ChainSlotView[]>([]);
export const selectedEffectIndex = writable<number | null>(null);
export const presets = writable<PresetSummary[]>([]);
export const monitorDeviceId = writable<string | null>(null);
export const monitorEnabled = writable<boolean>(false);
