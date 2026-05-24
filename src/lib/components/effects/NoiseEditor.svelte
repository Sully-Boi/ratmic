<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as {
    whiteAmount?: number;
    humAmount?: number;
    humHz?: number;
    crackleRate?: number;
    gateMode?: "always" | "onspeech";
    speechThresholdDb?: number;
  }) ?? {};

  async function setParam(key: string, value: number | string) {
    const next = { ...params, [key]: value };
    try {
      await ipc.setEffectParams(slot.index, next);
      chain.update((items) =>
        items.map((s) => (s.index === slot.index ? { ...s, params: next } : s))
      );
    } catch (e) {
      console.error(e);
    }
  }

  function handleGateModeChange(e: Event) {
    setParam("gateMode", (e.target as HTMLSelectElement).value);
  }
</script>

<Slider label="White"   value={params.whiteAmount ?? 0}    min={0}  max={0.5} step={0.01} onChange={(v) => setParam("whiteAmount", v)} />
<Slider label="Hum"     value={params.humAmount ?? 0}      min={0}  max={0.5} step={0.01} onChange={(v) => setParam("humAmount", v)} />
<Slider label="Hum Hz"  value={params.humHz ?? 60}         min={40} max={120} step={1} unit=" Hz" onChange={(v) => setParam("humHz", v)} />
<Slider label="Crackle" value={params.crackleRate ?? 0}    min={0}  max={50}  step={0.5} unit="/s" onChange={(v) => setParam("crackleRate", v)} />

<label class="row">
  <span>Gate</span>
  <select
    value={params.gateMode ?? "always"}
    on:change={handleGateModeChange}
  >
    <option value="always">always</option>
    <option value="onspeech">on speech</option>
  </select>
</label>

{#if params.gateMode === "onspeech"}
  <Slider label="Speech thresh" value={params.speechThresholdDb ?? -40} min={-90} max={0} step={1} unit=" dB" onChange={(v) => setParam("speechThresholdDb", v)} />
{/if}

<style>
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    font-size: 12px;
    color: var(--text-1);
    margin-bottom: 0.75rem;
  }
</style>
