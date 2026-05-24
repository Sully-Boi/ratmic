<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { ceilingDb?: number; releaseMs?: number }) ?? {};

  async function setParam(key: string, value: number) {
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
</script>

<Slider label="Ceiling" value={params.ceilingDb ?? -3} min={-24} max={0}   step={0.5} unit=" dB" onChange={(v) => setParam("ceilingDb", v)} />
<Slider label="Release" value={params.releaseMs ?? 80} min={1}   max={500} step={1}   unit=" ms" onChange={(v) => setParam("releaseMs", v)} />
