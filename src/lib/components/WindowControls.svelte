<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const win = getCurrentWindow();
  let isMaximized = false;
  let unlisten: (() => void) | null = null;

  async function refresh() {
    isMaximized = await win.isMaximized();
  }

  onMount(async () => {
    await refresh();
    // Listen for resize events to update the maximize icon state.
    unlisten = await win.onResized(() => refresh());
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function minimize() {
    await win.minimize();
  }
  async function toggleMaximize() {
    await win.toggleMaximize();
    await refresh();
  }
  async function close() {
    await win.close();
  }
</script>

<div class="controls">
  <button class="ctrl" on:click={minimize} aria-label="Minimize">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <path d="M0 5h10" stroke="currentColor" stroke-width="1.2" />
    </svg>
  </button>
  <button class="ctrl" on:click={toggleMaximize} aria-label="Maximize">
    {#if isMaximized}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <rect x="0.5" y="2.5" width="6" height="6" stroke="currentColor" stroke-width="1.0" />
        <rect x="3.5" y="0.5" width="6" height="6" stroke="currentColor" stroke-width="1.0" />
      </svg>
    {:else}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
        <rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" stroke-width="1.2" />
      </svg>
    {/if}
  </button>
  <button class="ctrl close" on:click={close} aria-label="Close">
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <path d="M0 0l10 10M10 0L0 10" stroke="currentColor" stroke-width="1.2" />
    </svg>
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-items: stretch;
    height: 100%;
  }
  .ctrl {
    width: 46px;
    height: 100%;
    background: transparent;
    border: none;
    color: var(--text-1);
    cursor: pointer;
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 0;
    transition: background 80ms ease, color 80ms ease;
  }
  .ctrl:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-0);
  }
  .ctrl.close:hover {
    background: var(--danger);
    color: #fff;
  }
</style>
