<script lang="ts">
  import { onMount } from "svelte";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { flip } from "svelte/animate";
  import { ipc, type ChainSlotView } from "../ipc";
  import { chain, selectedEffectIndex, engineRunning } from "../stores";
  import EffectChainRow from "./EffectChainRow.svelte";

  const ADDABLE_TYPES = [
    "gain", "bandpass", "bitcrusher", "clipper",
    "noise", "packetLoss", "noiseGate",
  ];
  let addType = "gain";
  let busy = false;

  // dnd items: the non-limiter slots, each given a stable id (their type+index).
  // We keep an ordered copy locally so dnd can reorder it; the limiter is excluded.
  type DndItem = ChainSlotView & { id: number };
  let dndItems: DndItem[] = [];
  let limiterSlot: ChainSlotView | null = null;

  function rebuildLocal(slots: ChainSlotView[]) {
    dndItems = slots
      .filter((s) => s.type_name !== "limiter")
      .map((s) => ({ ...s, id: s.index }));
    limiterSlot = slots.find((s) => s.type_name === "limiter") ?? null;
  }

  $: rebuildLocal($chain);

  async function refresh() {
    try {
      chain.set(await ipc.getChain());
    } catch (e) {
      console.error(e);
    }
  }

  function handleConsider(e: CustomEvent<DndEvent<DndItem>>) {
    dndItems = e.detail.items;
  }

  async function handleFinalize(e: CustomEvent<DndEvent<DndItem>>) {
    // Capture the order BEFORE applying the new one, then use the dragged item's
    // id (provided by dnd-action) to compute its old vs new position. Positions
    // in the non-limiter list map 1:1 to backend slot indices (0..k-1), since the
    // limiter is excluded here and is always the last backend slot.
    const oldOrder = dndItems.map((i) => i.id);
    dndItems = e.detail.items;
    const newOrder = dndItems.map((i) => i.id);
    const draggedId = Number(e.detail.info.id);
    const from = oldOrder.indexOf(draggedId);
    const to = newOrder.indexOf(draggedId);
    if (from === to || from < 0 || to < 0) {
      return;
    }
    busy = true;
    try {
      await ipc.reorderEffects(from, to);
      await refresh();
    } catch (err) {
      console.error(err);
      await refresh();
    } finally {
      busy = false;
    }
  }

  function toggleExpand(index: number) {
    selectedEffectIndex.update((cur) => (cur === index ? null : index));
  }

  async function remove(index: number) {
    busy = true;
    try {
      await ipc.removeEffect(index);
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

<div
  class="list"
  use:dndzone={{ items: dndItems, flipDurationMs: 200, dragDisabled: !$engineRunning || busy }}
  on:consider={handleConsider}
  on:finalize={handleFinalize}
>
  {#each dndItems as item (item.id)}
    <div animate:flip={{ duration: 200 }}>
      <EffectChainRow
        slot={item}
        expanded={$selectedEffectIndex === item.index}
        engineRunning={$engineRunning}
        onToggleExpand={() => toggleExpand(item.index)}
        onRemove={remove}
      />
    </div>
  {/each}
</div>

{#if limiterSlot}
  <div class="limiter-wrap">
    <EffectChainRow
      slot={limiterSlot}
      expanded={$selectedEffectIndex === limiterSlot.index}
      engineRunning={$engineRunning}
      onToggleExpand={() => limiterSlot && toggleExpand(limiterSlot.index)}
      onRemove={() => {}}
    />
  </div>
{/if}

{#if $engineRunning}
  <div class="add-row">
    <select bind:value={addType} disabled={busy}>
      {#each ADDABLE_TYPES as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
    <button on:click={add} disabled={busy}>+ Add effect</button>
  </div>
{:else}
  <p class="muted">Start the engine to edit the chain.</p>
{/if}

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-height: 8px;
  }
  .limiter-wrap { margin-top: 0.3rem; }
  .add-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.6rem;
  }
  select { flex: 1; }
  .muted { color: var(--text-2); font-size: 12px; margin-top: 0.6rem; }
</style>
