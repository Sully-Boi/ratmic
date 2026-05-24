<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { bitDepth?: number; sampleRateHz?: number; mix?: number }) ?? {};

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

<Slider label="Bit depth"   value={params.bitDepth ?? 16}      min={1}   max={16}    step={1}    unit=" bit"  onChange={(v) => setParam("bitDepth", v)} />
<Slider label="Sample rate" value={params.sampleRateHz ?? 48000} min={1000} max={48000} step={100}  unit=" Hz"  onChange={(v) => setParam("sampleRateHz", v)} />
<Slider label="Mix"         value={params.mix ?? 1.0}           min={0}   max={1}     step={0.05} onChange={(v) => setParam("mix", v)} />
