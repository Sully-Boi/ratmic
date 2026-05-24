<script lang="ts">
  import type { ChainSlotView } from "../ipc";
  import { ipc } from "../ipc";
  import { chain } from "../stores";
  import PillToggle from "./PillToggle.svelte";
  import EffectIcon from "./EffectIcon.svelte";
  import { displayName, categoryColor } from "../format/effect-meta";
  import { summarizeParams } from "../format/effect-params";
  import GainEditor from "./effects/GainEditor.svelte";
  import BandpassEditor from "./effects/BandpassEditor.svelte";
  import BitcrusherEditor from "./effects/BitcrusherEditor.svelte";
  import ClipperEditor from "./effects/ClipperEditor.svelte";
  import NoiseEditor from "./effects/NoiseEditor.svelte";
  import PacketLossEditor from "./effects/PacketLossEditor.svelte";
  import NoiseGateEditor from "./effects/NoiseGateEditor.svelte";
  import LimiterEditor from "./effects/LimiterEditor.svelte";

  export let slot: ChainSlotView;
  export let expanded: boolean = false;
  export let engineRunning: boolean = false;
  export let onToggleExpand: () => void;
  export let onRemove: (index: number) => void;

  $: isLimiter = slot.type_name === "limiter";
  $: summary = summarizeParams(slot.type_name, slot.params);
  $: accent = categoryColor(slot.type_name);

  async function toggleEnabled(v: boolean) {
    try {
      await ipc.setEffectEnabled(slot.index, v);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, enabled: v } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="row" class:expanded class:fixed={isLimiter} style="--accent: {accent}">
  <div class="head">
    {#if isLimiter}
      <span class="grip locked" title="Limiter is fixed">●</span>
    {:else}
      <span class="grip" title="Drag to reorder">⋮⋮</span>
    {/if}

    <EffectIcon typeName={slot.type_name} />

    <button class="namebtn" on:click={onToggleExpand} aria-expanded={expanded}>
      <span class="name">{displayName(slot.type_name)}</span>
      <span class="summary tabular">{summary}</span>
    </button>

    <PillToggle
      checked={slot.enabled}
      disabled={!engineRunning || isLimiter}
      ariaLabel={`enable ${slot.type_name}`}
      onChange={toggleEnabled}
    />

    <button class="chev" on:click={onToggleExpand} aria-label="Toggle parameters">
      {expanded ? "⌃" : "⌄"}
    </button>

    {#if isLimiter}
      <span class="lock" title="Fixed safety limiter">🔒</span>
    {:else}
      <button class="x" title="Remove" on:click={() => onRemove(slot.index)} disabled={!engineRunning}>×</button>
    {/if}
  </div>

  {#if expanded}
    <div class="params">
      {#if slot.type_name === "gain"}<GainEditor {slot} />
      {:else if slot.type_name === "bandpass"}<BandpassEditor {slot} />
      {:else if slot.type_name === "bitcrusher"}<BitcrusherEditor {slot} />
      {:else if slot.type_name === "clipper"}<ClipperEditor {slot} />
      {:else if slot.type_name === "noise"}<NoiseEditor {slot} />
      {:else if slot.type_name === "packetLoss"}<PacketLossEditor {slot} />
      {:else if slot.type_name === "noiseGate"}<NoiseGateEditor {slot} />
      {:else if slot.type_name === "limiter"}<LimiterEditor {slot} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .row {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .row.expanded {
    border-left: 2px solid var(--accent);
  }
  .row.fixed {
    background: repeating-linear-gradient(
      45deg, var(--bg-2), var(--bg-2) 6px, var(--bg-1) 6px, var(--bg-1) 12px
    );
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
  }
  .grip {
    color: var(--text-2);
    cursor: grab;
    font-size: 12px;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }
  .grip.locked { cursor: default; }
  .namebtn {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }
  .name {
    font-weight: 600;
    color: var(--text-0);
    flex-shrink: 0;
  }
  .summary {
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev {
    background: transparent;
    border: none;
    color: var(--text-1);
    cursor: pointer;
    font-size: 12px;
    padding: 0 0.25rem;
    flex-shrink: 0;
  }
  .x {
    width: 22px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--text-1);
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 1px 0;
    flex-shrink: 0;
  }
  .x:hover:not(:disabled) { background: var(--danger); color: #fff; }
  .lock { font-size: 11px; flex-shrink: 0; opacity: 0.7; }
  .params {
    padding: 0.6rem 0.75rem 0.2rem;
    border-top: 1px solid var(--border);
    background: var(--bg-1);
  }
  @media (max-width: 620px) {
    .summary { display: none; }
  }
</style>
