<script lang="ts">
  import { categoryColor } from "../format/effect-meta";
  export let typeName: string;
  $: color = categoryColor(typeName);
</script>

<span class="icon" style="color: {color}; background: color-mix(in srgb, {color} 16%, transparent);">
  {#if typeName === "gain"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M2 11l4-6 4 4 4-7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "bandpass"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 8h3c1-4 7-4 8 0h3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "bitcrusher"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 11h3V7h3v6h3V5h3" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "clipper"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 12V4h5v8h4V4h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "noise"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 8l2-3 2 6 2-8 2 9 2-5 2 4 2-3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "packetLoss"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><rect x="1" y="5" width="2" height="6" fill="currentColor"/><rect x="7" y="5" width="2" height="6" fill="currentColor"/><rect x="13" y="5" width="2" height="6" fill="currentColor"/></svg>
  {:else if typeName === "noiseGate"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 12h5V4h4v8h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else if typeName === "limiter"}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><path d="M1 4h14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M3 11l2-3 2 2 2-4 2 5" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {:else}
    <svg viewBox="0 0 16 16" width="12" height="12" fill="none"><circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="1.6"/></svg>
  {/if}
</span>

<style>
  .icon {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 5px;
    flex-shrink: 0;
  }
</style>
