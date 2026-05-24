<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, type DeviceInfo } from "../ipc";
  import { inputDeviceId, outputDeviceId, monitorDeviceId, monitorEnabled, engineRunning, showSetup } from "../stores";
  import PillToggle from "./PillToggle.svelte";

  let inputs: DeviceInfo[] = [];
  let outputs: DeviceInfo[] = [];
  let loadError = "";

  async function refresh() {
    loadError = "";
    try {
      const all = await ipc.listDevices();
      inputs = all.filter((d) => d.kind === "Input");
      outputs = all.filter((d) => d.kind === "Output");
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(refresh);

  async function handleListenToggle(v: boolean) {
    monitorEnabled.set(v);
    if ($engineRunning) {
      try {
        await ipc.setMonitorEnabled(v);
      } catch (e) {
        // engine may have stopped between check and call — ignore
      }
    }
  }

  $: listenDisabled = !$monitorDeviceId || !$engineRunning;
</script>

<div class="strip">
  <label>
    <span class="lbl">Input</span>
    <select bind:value={$inputDeviceId}>
      <option value={null}>— select —</option>
      {#each inputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>

  <label>
    <span class="lbl">Output</span>
    <select bind:value={$outputDeviceId}>
      <option value={null}>— select —</option>
      {#each outputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>

  <label>
    <span class="lbl">Monitor</span>
    <select bind:value={$monitorDeviceId}>
      <option value={null}>— none —</option>
      {#each outputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>

  <div class="listen-group">
    <span class="lbl">Listen</span>
    <PillToggle
      checked={$monitorEnabled}
      disabled={listenDisabled}
      ariaLabel="Enable monitor listen-back"
      onChange={handleListenToggle}
    />
  </div>

  <button class="refresh-btn" on:click={refresh} title="Refresh device list">↻</button>
  <button class="setup-btn" on:click={() => showSetup.set(true)} title="Open setup wizard">?</button>
  {#if loadError}<span class="err">{loadError}</span>{/if}
</div>

<style>
  .strip {
    height: 40px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0 0.75rem;
    overflow: hidden;
    user-select: none;
  }
  label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    font-size: 12px;
    color: var(--text-1);
    min-width: 0;
    flex: 1 1 0;
  }
  .lbl {
    flex-shrink: 0;
    color: var(--text-2);
  }
  select {
    min-width: 0;
    max-width: 200px;
    flex: 1 1 0;
    text-overflow: ellipsis;
    font-size: 12px;
  }
  .listen-group {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 12px;
    color: var(--text-1);
    flex-shrink: 0;
  }
  .refresh-btn, .setup-btn {
    flex-shrink: 0;
    font-size: 14px;
    padding: 0 0.5rem;
    height: 26px;
    line-height: 1;
  }
  .setup-btn {
    font-weight: 700;
    min-width: 26px;
  }
  .err {
    color: var(--danger);
    font-size: 12px;
    flex-shrink: 0;
  }
</style>
