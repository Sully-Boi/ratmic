<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { gainDb?: number }) ?? {};

  async function setGainDb(value: number) {
    const next = { ...params, gainDb: value };
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

<Slider
  label="Gain"
  value={params.gainDb ?? 0}
  min={-24}
  max={24}
  step={0.5}
  unit=" dB"
  onChange={setGainDb}
/>
