<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { thresholdDb?: number; attackMs?: number; releaseMs?: number; chatterAmount?: number }) ?? {};

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

<Slider label="Threshold" value={params.thresholdDb ?? -40} min={-60} max={0}   step={1}    unit=" dB" onChange={(v) => setParam("thresholdDb", v)} />
<Slider label="Attack"    value={params.attackMs ?? 5}      min={0.5} max={200} step={0.5}  unit=" ms" onChange={(v) => setParam("attackMs", v)} />
<Slider label="Release"   value={params.releaseMs ?? 80}    min={0.5} max={500} step={1}    unit=" ms" onChange={(v) => setParam("releaseMs", v)} />
<Slider label="Chatter"   value={params.chatterAmount ?? 0} min={0}   max={1}   step={0.05} onChange={(v) => setParam("chatterAmount", v)} />
