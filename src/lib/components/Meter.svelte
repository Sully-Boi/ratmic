<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  export let label: string;
  export let peakDb: number = -90;
  export let rmsDb: number = -90;

  const SEGMENTS = 20;
  /**
   * Map dB to a normalized [0, 1] segment fill where the loud half (-24 to 0 dB)
   * occupies ~60% of the bar.
   */
  function dbToNorm(db: number): number {
    if (db <= -60) return 0;
    if (db >= 0) return 1;
    if (db >= -24) {
      // -24 to 0 → 0.4 to 1.0
      return 0.4 + ((db + 24) / 24) * 0.6;
    } else {
      // -60 to -24 → 0 to 0.4
      return ((db + 60) / 36) * 0.4;
    }
  }

  let peakHold = -90;
  let peakHoldDecayTimer: number | null = null;

  $: if (peakDb > peakHold) {
    peakHold = peakDb;
  }

  function startDecayLoop() {
    const tick = () => {
      // Decay 60 dB over 1000 ms = 60 dB/sec.
      if (peakHold > peakDb) {
        peakHold = Math.max(peakDb, peakHold - 60 / 60); // ~1 dB per frame at 60 fps
      }
      peakHoldDecayTimer = requestAnimationFrame(tick);
    };
    peakHoldDecayTimer = requestAnimationFrame(tick);
  }

  onMount(startDecayLoop);
  onDestroy(() => {
    if (peakHoldDecayTimer !== null) cancelAnimationFrame(peakHoldDecayTimer);
  });

  $: rmsNorm = dbToNorm(rmsDb);
  $: peakHoldNorm = dbToNorm(peakHold);
  $: clipping = peakDb > -0.5;

  function segmentColor(idx: number): string {
    const t = idx / SEGMENTS;
    if (t < 0.7) return "var(--ok)";
    if (t < 0.9) return "var(--warn)";
    return "var(--danger)";
  }
</script>

<div class="meter">
  <span class="label">{label}</span>
  <div class="bar">
    {#each Array(SEGMENTS) as _, i}
      {@const filled = i / SEGMENTS < rmsNorm}
      {@const isHold = Math.abs(i / SEGMENTS - peakHoldNorm) < 1 / SEGMENTS}
      <div
        class="seg"
        class:filled
        class:hold={isHold && !filled}
        style="background: {filled ? segmentColor(i) : 'var(--bg-2)'}"
      />
    {/each}
  </div>
  <span class="value tabular" class:clipping>{peakDb.toFixed(1)} dB</span>
  <span class="led" class:on={clipping} aria-hidden="true" />
</div>

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 11px;
    min-width: 0;
    flex: 1 1 220px;
  }
  .label {
    color: var(--text-1);
    width: 26px;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  .bar {
    flex: 1;
    height: 12px;
    display: flex;
    gap: 1px;
  }
  .seg {
    flex: 1;
    border-radius: 1px;
    transition: background 30ms linear;
  }
  .seg.hold {
    background: var(--text-0) !important;
  }
  .value {
    color: var(--text-1);
    width: 58px;
    text-align: right;
    font-weight: 600;
  }
  .value.clipping {
    color: var(--danger);
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(220, 38, 38, 0.2);
    flex-shrink: 0;
    transition: background 80ms ease, box-shadow 80ms ease;
  }
  .led.on {
    background: var(--danger);
    box-shadow: 0 0 6px var(--danger);
  }
</style>
