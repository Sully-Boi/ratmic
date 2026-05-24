<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, type DeviceInfo } from "../ipc";
  import { inputDeviceId, outputDeviceId } from "../stores";

  let inputs: DeviceInfo[] = [];
  let outputs: DeviceInfo[] = [];
  let loadError = "";

  async function refresh() {
    try {
      const all = await ipc.listDevices();
      inputs = all.filter((d) => d.kind === "Input");
      outputs = all.filter((d) => d.kind === "Output");
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(refresh);
</script>

<div class="bar">
  <label>
    Input
    <select bind:value={$inputDeviceId}>
      <option value={null}>— select —</option>
      {#each inputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>
  <label>
    Output
    <select bind:value={$outputDeviceId}>
      <option value={null}>— select —</option>
      {#each outputs as d}
        <option value={d.id}>{d.name}{d.is_default ? " (default)" : ""}</option>
      {/each}
    </select>
  </label>
  <button on:click={refresh}>Refresh</button>
  {#if loadError}<span class="err">{loadError}</span>{/if}
</div>

<style>
  .bar {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    min-width: 0;
    flex: 1;
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
  select {
    min-width: 0;
    max-width: 220px;
    flex: 1 1 0;
    text-overflow: ellipsis;
  }
  button {
    flex-shrink: 0;
  }
  .err {
    color: var(--danger);
    font-size: 12px;
    flex-shrink: 0;
  }
</style>
