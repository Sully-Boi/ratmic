<script lang="ts">
  export let checked: boolean = false;
  export let disabled: boolean = false;
  export let onChange: (v: boolean) => void = () => {};
  export let ariaLabel: string = "toggle";

  function toggle() {
    if (disabled) return;
    onChange(!checked);
  }

  function handleKey(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      onChange(!checked);
    }
  }
</script>

<button
  type="button"
  class="pill"
  class:on={checked}
  class:disabled
  aria-pressed={checked}
  aria-label={ariaLabel}
  disabled={disabled}
  on:click|stopPropagation={toggle}
  on:keydown={handleKey}
>
  <span class="dot" />
</button>

<style>
  .pill {
    --w: 32px;
    --h: 18px;
    width: var(--w);
    height: var(--h);
    background: var(--bg-3);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0;
    position: relative;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .pill:hover:not(.disabled) { background: #353540; }
  .pill.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .pill.on:hover { background: var(--accent-hot); }
  .pill.disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .dot {
    position: absolute;
    top: 1px;
    left: 1px;
    width: 14px;
    height: 14px;
    background: var(--text-0);
    border-radius: 50%;
    transition: transform 120ms ease;
  }
  .pill.on .dot {
    transform: translateX(calc(var(--w) - var(--h)));
  }
</style>
