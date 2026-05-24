<script lang="ts">
  import { chain, selectedEffectIndex } from "../stores";
  import GainEditor from "./effects/GainEditor.svelte";
  import BandpassEditor from "./effects/BandpassEditor.svelte";
  import BitcrusherEditor from "./effects/BitcrusherEditor.svelte";
  import ClipperEditor from "./effects/ClipperEditor.svelte";
  import NoiseEditor from "./effects/NoiseEditor.svelte";
  import PacketLossEditor from "./effects/PacketLossEditor.svelte";
  import NoiseGateEditor from "./effects/NoiseGateEditor.svelte";
  import LimiterEditor from "./effects/LimiterEditor.svelte";

  $: slot = $selectedEffectIndex !== null
    ? $chain.find((s) => s.index === $selectedEffectIndex)
    : null;
</script>

<h3>Parameters</h3>

{#if !slot}
  <p class="muted">Select an effect to edit its parameters.</p>
{:else if slot.type_name === "gain"}
  <GainEditor {slot} />
{:else if slot.type_name === "bandpass"}
  <BandpassEditor {slot} />
{:else if slot.type_name === "bitcrusher"}
  <BitcrusherEditor {slot} />
{:else if slot.type_name === "clipper"}
  <ClipperEditor {slot} />
{:else if slot.type_name === "noise"}
  <NoiseEditor {slot} />
{:else if slot.type_name === "packetLoss"}
  <PacketLossEditor {slot} />
{:else if slot.type_name === "noiseGate"}
  <NoiseGateEditor {slot} />
{:else if slot.type_name === "limiter"}
  <LimiterEditor {slot} />
{:else}
  <p class="muted">No editor for "{slot.type_name}" yet.</p>
{/if}

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  .muted { color: var(--text-2); font-size: 12px; }
</style>
