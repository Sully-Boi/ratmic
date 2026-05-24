<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { dropChance?: number; minDropMs?: number; maxDropMs?: number; stutterChance?: number }) ?? {};

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

<Slider label="Drop chance"    value={params.dropChance ?? 0}     min={0}  max={0.5} step={0.01} onChange={(v) => setParam("dropChance", v)} />
<Slider label="Min drop"       value={params.minDropMs ?? 30}     min={10} max={500} step={5}  unit=" ms" onChange={(v) => setParam("minDropMs", v)} />
<Slider label="Max drop"       value={params.maxDropMs ?? 140}    min={10} max={500} step={5}  unit=" ms" onChange={(v) => setParam("maxDropMs", v)} />
<Slider label="Stutter chance" value={params.stutterChance ?? 0}  min={0}  max={0.3} step={0.01} onChange={(v) => setParam("stutterChance", v)} />
