<script lang="ts">
  export let label: string;
  export let peakDb: number = -90;
  export let rmsDb: number = -90;

  $: peakPct = dbToPct(peakDb);
  $: rmsPct = dbToPct(rmsDb);
  $: clipping = peakDb > -0.5;

  function dbToPct(db: number): number {
    // Map -60 dB ... 0 dB to 0 ... 100 %.
    const clamped = Math.max(-60, Math.min(0, db));
    return ((clamped + 60) / 60) * 100;
  }
</script>

<div class="meter">
  <span class="label">{label}</span>
  <div class="track">
    <div class="rms" style="width: {rmsPct}%"></div>
    <div class="peak" class:clipping style="left: {peakPct}%"></div>
  </div>
  <span class="value">{peakDb.toFixed(1)} dB</span>
</div>

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 11px;
    min-width: 220px;
  }
  .label {
    color: var(--text-1);
    width: 44px;
  }
  .track {
    position: relative;
    flex: 1;
    height: 10px;
    background: var(--bg-2);
    border-radius: 3px;
    overflow: hidden;
  }
  .rms {
    height: 100%;
    background: linear-gradient(90deg, var(--ok), var(--warn), var(--danger));
    transition: width 30ms linear;
  }
  .peak {
    position: absolute;
    top: 0;
    width: 2px;
    height: 100%;
    background: var(--text-0);
    transition: left 30ms linear;
  }
  .peak.clipping {
    background: var(--danger);
    box-shadow: 0 0 4px var(--danger);
  }
  .value {
    color: var(--text-2);
    width: 50px;
    text-align: right;
  }
</style>
