<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import TitleBar from "./lib/components/TitleBar.svelte";
  import DeviceStrip from "./lib/components/DeviceStrip.svelte";
  import Meter from "./lib/components/Meter.svelte";
  import EffectChain from "./lib/components/EffectChain.svelte";
  import PresetSidebar from "./lib/components/PresetSidebar.svelte";
  import SetupWizard from "./lib/components/SetupWizard.svelte";
  import UpdatePrompt from "./lib/components/UpdatePrompt.svelte";
  import { ipc, events } from "./lib/ipc";
  import {
    settings,
    inputDeviceId,
    outputDeviceId,
    engineRunning,
    meters,
    engineError,
    monitorDeviceId,
    monitorEnabled,
    showSetup,
    presets,
    chain,
    effectsEnabled,
    hotkey,
  } from "./lib/stores";
  import { isVirtualCable } from "./lib/format/devices";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let unsubs: UnlistenFn[] = [];

  // Task 2: Device-disconnect watcher state.
  let deviceWatch: ReturnType<typeof setInterval> | null = null;

  async function checkDevices() {
    try {
      const devs = await ipc.listDevices();
      const ids = new Set(devs.map((d) => d.id));
      if ($inputDeviceId && !ids.has($inputDeviceId)) {
        await failDevice("Input", $inputDeviceId);
      } else if ($outputDeviceId && !ids.has($outputDeviceId)) {
        await failDevice("Output", $outputDeviceId);
      } else if ($monitorEnabled && $monitorDeviceId && !ids.has($monitorDeviceId)) {
        // Monitor gone: don't kill the whole engine, just warn.
        engineError.set(`Monitor device disconnected (${$monitorDeviceId})`);
      }
    } catch (_) {
      // listing failed transiently — ignore this tick
    }
  }

  async function failDevice(kind: string, id: string) {
    try {
      await ipc.stopEngine();
    } catch (_) {
      /* ignore */
    }
    engineError.set(`${kind} device disconnected (${id}) — engine stopped`);
  }

  // Start/stop the watcher when engine state changes.
  $: {
    if ($engineRunning && deviceWatch === null) {
      deviceWatch = setInterval(checkDevices, 2000);
    } else if (!$engineRunning && deviceWatch !== null) {
      clearInterval(deviceWatch);
      deviceWatch = null;
    }
  }

  // setupOpen follows the showSetup store; closing also writes to the store.
  $: setupOpen = $showSetup;

  onMount(async () => {
    const s = await ipc.loadSettings();
    settings.set(s);
    if (s.input_device_id) inputDeviceId.set(s.input_device_id);
    if (s.output_device_id) outputDeviceId.set(s.output_device_id);
    if (s.monitor_device_id) monitorDeviceId.set(s.monitor_device_id);
    monitorEnabled.set(s.monitor_enabled);
    hotkey.set(s.hotkey ?? null);
    effectsEnabled.set(await ipc.effectsEnabled());

    engineRunning.set(await ipc.engineRunning());

    // Smart defaults: only set if nothing was restored from saved settings.
    const devices = await ipc.listDevices();
    if (!s.input_device_id) {
      const def = devices.find((d) => d.kind === "Input" && d.is_default) ?? devices.find((d) => d.kind === "Input");
      if (def) inputDeviceId.set(def.id);
    }
    if (!s.output_device_id) {
      const cable = devices.find((d) => d.kind === "Output" && isVirtualCable(d.name));
      if (cable) outputDeviceId.set(cable.id);
    }

    // First-run: open wizard if not yet seen.
    if (!s.onboarding_seen) showSetup.set(true);

    unsubs.push(await events.onMeters((m) => meters.set(m)));
    unsubs.push(
      await events.onEngineState((s) => {
        engineRunning.set(s.running);
        engineError.set(s.error);
      })
    );
    unsubs.push(await events.onEffectsState((e) => effectsEnabled.set(e.enabled)));
  });

  async function closeSetup() {
    showSetup.set(false);
    const s = await ipc.loadSettings();
    s.onboarding_seen = true;
    await ipc.saveSettings(s);
  }

  onDestroy(() => {
    unsubs.forEach((u) => u());
    if (deviceWatch !== null) clearInterval(deviceWatch);
  });

  async function toggleEngine() {
    engineError.set(null);
    try {
      if ($engineRunning) {
        await ipc.stopEngine();
      } else {
        if (!$inputDeviceId || !$outputDeviceId) {
          engineError.set("pick input and output devices first");
          return;
        }
        await ipc.startEngine($inputDeviceId, $outputDeviceId, $monitorDeviceId, $monitorEnabled);
        // Sync monitor enabled state in case engine state drifted
        await ipc.setMonitorEnabled($monitorEnabled);
        const s = await ipc.loadSettings();
        s.input_device_id = $inputDeviceId;
        s.output_device_id = $outputDeviceId;
        s.monitor_device_id = $monitorDeviceId;
        s.monitor_enabled = $monitorEnabled;
        await ipc.saveSettings(s);
        // Task 1: Auto-restore the last-used preset so the app resumes where it left off.
        if (s.last_preset_name) {
          const match = $presets.find((p) => p.name === s.last_preset_name);
          if (match) {
            try {
              await ipc.loadPreset(s.last_preset_name, match.builtin);
              chain.set(await ipc.getChain());
            } catch (_) {
              // Preset may have been deleted since last session — ignore.
            }
          }
        }
      }
    } catch (e) {
      engineError.set(String(e));
    }
  }
</script>

<div class="shell">
  <TitleBar />
  <DeviceStrip />

  <main class="body">
    <aside class="sidebar"><PresetSidebar /></aside>
    <section class="chain"><EffectChain /></section>
  </main>

  <footer class="bottom-bar">
    <Meter label="IN"  peakDb={$meters.input_peak_db}  rmsDb={$meters.input_rms_db} />
    <Meter label="OUT" peakDb={$meters.output_peak_db} rmsDb={$meters.output_rms_db} />
    <div class="lim" class:active={$meters.limiter_activity_pct > 0}>
      <span class="dot" />
      <span>LIM</span>
      <span class="pct tabular">{Math.round($meters.limiter_activity_pct)}%</span>
    </div>
    <div class="fx" class:bypassed={!$effectsEnabled}>
      {$effectsEnabled ? "FX ON" : "BYPASSED"}
    </div>
    <div class="spacer"></div>
    {#if $engineError}<span class="err">{$engineError}</span>{/if}
    <button
      class="engine-btn"
      class:running={$engineRunning}
      on:click={toggleEngine}
    >
      {$engineRunning ? "■ STOP" : "▶ START"}
    </button>
  </footer>
</div>

<SetupWizard open={setupOpen} onClose={closeSetup} />
<UpdatePrompt />

<style>
  .shell { display: grid; grid-template-rows: 36px 40px 1fr 64px; height: 100%; }
  .body { display: grid; grid-template-columns: 220px 1fr; min-height: 0; }
  .sidebar, .chain { padding: 0.75rem; overflow: auto; }
  .sidebar { background: var(--bg-1); border-right: 1px solid var(--border); }
  .bottom-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0 1rem;
    background: var(--bg-1);
    border-top: 1px solid var(--border);
    height: 64px;
    min-width: 0;
    overflow: hidden;
  }
  .spacer { flex: 1; min-width: 0; }
  .fx {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--accent);
    flex-shrink: 0;
  }
  .fx.bypassed { color: var(--text-2); }
  .err {
    color: var(--danger);
    font-size: 12px;
    margin-right: 0.5rem;
    flex-shrink: 0;
  }
  .lim {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-2);
    letter-spacing: 0.05em;
    flex-shrink: 0;
  }
  .lim .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-2);
    transition: background 80ms ease, box-shadow 80ms ease;
  }
  .lim.active { color: var(--accent); }
  .lim.active .dot {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
  }
  .lim .pct {
    min-width: 32px;
    text-align: right;
    color: var(--text-1);
  }
  .engine-btn {
    min-width: 120px;
    height: 40px;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 0.05em;
    background: var(--accent);
    color: #1a1a1a;
    border-color: var(--accent);
    flex-shrink: 0;
  }
  .engine-btn:hover:not(:disabled) {
    background: var(--accent-hot);
    border-color: var(--accent-hot);
  }
  .engine-btn.running {
    background: transparent;
    color: var(--danger);
    border-color: var(--danger);
  }
  .engine-btn.running:hover:not(:disabled) {
    background: var(--danger);
    color: #fff;
  }
</style>
