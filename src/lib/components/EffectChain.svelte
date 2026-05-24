<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../ipc";
  import { chain, selectedEffectIndex, engineRunning } from "../stores";
  import PillToggle from "./PillToggle.svelte";

  const ADDABLE_TYPES = [
    "gain", "bandpass", "bitcrusher", "clipper",
    "noise", "packetLoss", "noiseGate",
  ];

  let addType = "gain";
  let busy = false;

  async function refresh() {
    try {
      chain.set(await ipc.getChain());
    } catch (e) {
      console.error(e);
    }
  }

  async function toggle(index: number, checked: boolean) {
    try {
      await ipc.setEffectEnabled(index, checked);
      await refresh();
    } catch (err) {
      console.error(err);
    }
  }

  async function remove(index: number) {
    busy = true;
    try {
      await ipc.removeEffect(index);
      // If the removed slot was selected, clear selection.
      if ($selectedEffectIndex === index) selectedEffectIndex.set(null);
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      busy = false;
    }
  }

  async function add() {
    busy = true;
    try {
      await ipc.addEffect(addType);
      await refresh();
    } catch (e) {
      console.error(e);
    } finally {
      busy = false;
    }
  }

  $: if ($engineRunning) refresh();

  onMount(refresh);
</script>

<h3>Effect Chain</h3>
<ul class="list">
  {#each $chain as slot}
    <li
      class:selected={$selectedEffectIndex === slot.index}
      class:fixed={slot.type_name === "limiter"}
      on:click={() => selectedEffectIndex.set(slot.index)}
      on:keydown={(e) => {
        if (e.key === "Enter" || e.key === " ") selectedEffectIndex.set(slot.index);
      }}
      role="button"
      tabindex="0"
    >
      <PillToggle
        checked={slot.enabled}
        disabled={!$engineRunning || slot.type_name === "limiter"}
        onChange={(v) => toggle(slot.index, v)}
        ariaLabel={`enable ${slot.type_name}`}
      />
      <span class="name">{slot.type_name}</span>
      {#if slot.type_name === "limiter"}
        <span class="badge">fixed</span>
      {:else}
        <button
          class="x"
          title="remove"
          on:click|stopPropagation={() => remove(slot.index)}
          disabled={!$engineRunning || busy}
        >
          ×
        </button>
      {/if}
    </li>
  {/each}
</ul>

{#if $engineRunning}
  <div class="add-row">
    <select bind:value={addType} disabled={busy}>
      {#each ADDABLE_TYPES as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
    <button on:click={add} disabled={busy}>+ Add</button>
  </div>
{:else}
  <p class="muted">Start the engine to edit the chain.</p>
{/if}

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }
  li:hover { background: var(--bg-3); }
  li.selected { border-color: var(--accent); }
  li.fixed { opacity: 0.85; }
  .name {
    flex: 1;
    text-transform: capitalize;
  }
  .badge {
    font-size: 10px;
    color: var(--text-2);
    padding: 1px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .x {
    width: 24px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-1);
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 2px 0;
  }
  .x:hover { background: var(--danger); color: white; }
  .add-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  select { flex: 1; }
  .muted { color: var(--text-2); font-size: 12px; margin-top: 0.5rem; }
</style>
