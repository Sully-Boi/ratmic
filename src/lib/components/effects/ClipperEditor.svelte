<script lang="ts">
  import { ipc, type ChainSlotView } from "../../ipc";
  import { chain } from "../../stores";
  import Slider from "./Slider.svelte";

  export let slot: ChainSlotView;

  $: params = (slot.params as { drive?: number; hardClip?: number; softClip?: number; outputTrimDb?: number }) ?? {};

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

<Slider label="Drive"     value={params.drive ?? 1.0}         min={1}   max={10}  step={0.1}  onChange={(v) => setParam("drive", v)} />
<Slider label="Hard clip" value={params.hardClip ?? 1.0}      min={0.1} max={1}   step={0.05} onChange={(v) => setParam("hardClip", v)} />
<Slider label="Soft clip" value={params.softClip ?? 0.0}      min={0}   max={1}   step={0.05} onChange={(v) => setParam("softClip", v)} />
<Slider label="Trim"      value={params.outputTrimDb ?? 0}    min={-24} max={6}   step={0.5}  unit=" dB" onChange={(v) => setParam("outputTrimDb", v)} />
