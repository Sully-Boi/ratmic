<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { lowCutHz?: number; highCutHz?: number; midBoostDb?: number }) ?? {};

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

<Slider label="Low cut"  value={params.lowCutHz ?? 100}  min={20}    max={8000}  step={10}  unit=" Hz" onChange={(v) => setParam("lowCutHz", v)} />
<Slider label="High cut" value={params.highCutHz ?? 8000} min={200}  max={20000} step={50}  unit=" Hz" onChange={(v) => setParam("highCutHz", v)} />
<Slider label="Mid boost" value={params.midBoostDb ?? 0} min={-12}  max={12}    step={0.5} unit=" dB" onChange={(v) => setParam("midBoostDb", v)} />
