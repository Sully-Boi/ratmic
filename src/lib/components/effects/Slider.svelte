<script lang="ts">
  export let label: string;
  export let value: number;
  export let min: number;
  export let max: number;
  export let step: number = 0.1;
  export let unit: string = "";
  export let onChange: (v: number) => void;

  let editing = false;
  let editValue = "";
  let dragging = false;

  function formatValue(v: number): string {
    if (step >= 1) return v.toFixed(0);
    if (step >= 0.1) return v.toFixed(1);
    return v.toFixed(2);
  }

  function handleInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    onChange(v);
  }

  function startEditing() {
    editing = true;
    editValue = formatValue(value);
  }

  function commitEdit() {
    const parsed = parseFloat(editValue);
    if (!Number.isNaN(parsed)) {
      const clamped = Math.max(min, Math.min(max, parsed));
      onChange(clamped);
    }
    editing = false;
  }

  function handleEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") commitEdit();
    if (e.key === "Escape") {
      editing = false;
    }
  }
</script>

<label class="slider">
  <span class="row">
    <span class="label">{label}</span>
    {#if editing}
      <input
        class="numeric-edit tabular"
        type="text"
        bind:value={editValue}
        on:blur={commitEdit}
        on:keydown={handleEditKey}
        autofocus
      />
    {:else}
      <button
        type="button"
        class="value tabular"
        on:click={startEditing}
        tabindex="0"
      >
        {formatValue(value)}{unit}
      </button>
    {/if}
  </span>
  <div class="track-container" class:dragging>
    <input
      type="range"
      {min}
      {max}
      {step}
      {value}
      on:input={handleInput}
      on:mousedown={() => (dragging = true)}
      on:mouseup={() => (dragging = false)}
      on:blur={() => (dragging = false)}
    />
    {#if dragging}
      <span
        class="tooltip tabular"
        style="left: {((value - min) / (max - min)) * 100}%"
      >
        {formatValue(value)}{unit}
      </span>
    {/if}
  </div>
</label>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-bottom: 0.85rem;
    font-size: 12px;
    color: var(--text-1);
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .label { font-weight: 500; }
  .value {
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-0);
    padding: 1px 6px;
    border-radius: 4px;
    font: inherit;
    font-weight: 600;
    cursor: text;
  }
  .value:hover { border-color: var(--border); }
  .numeric-edit {
    width: 80px;
    text-align: right;
    font-weight: 600;
  }
  .track-container {
    position: relative;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 4px;
    background: var(--bg-3);
    border-radius: 2px;
    outline: none;
    margin: 0;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    background: var(--accent);
    border-radius: 50%;
    cursor: grab;
    border: 2px solid var(--bg-1);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
  }
  input[type="range"]::-webkit-slider-thumb:active {
    cursor: grabbing;
    background: var(--accent-hot);
  }
  input[type="range"]::-moz-range-thumb {
    width: 14px;
    height: 14px;
    background: var(--accent);
    border-radius: 50%;
    border: 2px solid var(--bg-1);
    cursor: grab;
  }
  .tooltip {
    position: absolute;
    bottom: 20px;
    transform: translateX(-50%);
    background: var(--bg-3);
    color: var(--text-0);
    padding: 3px 7px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    pointer-events: none;
    white-space: nowrap;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
  }
</style>
