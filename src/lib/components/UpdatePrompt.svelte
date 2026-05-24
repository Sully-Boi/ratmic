<script lang="ts">
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { onMount } from "svelte";

  let update: Update | null = null;
  let status: "idle" | "downloading" | "error" = "idle";
  let dismissed = false;
  let errorMsg = "";

  onMount(async () => {
    try {
      const result = await check();
      if (result) {
        update = result;
      }
    } catch (e) {
      // Offline or no manifest yet — silently ignore on launch.
      console.warn("update check failed:", e);
    }
  });

  async function install() {
    if (!update) return;
    status = "downloading";
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      status = "error";
      errorMsg = String(e);
    }
  }
</script>

{#if update && !dismissed}
  <div class="banner">
    <div class="info">
      <strong>Update available — v{update.version}</strong>
      {#if status === "error"}<span class="err">{errorMsg}</span>{/if}
    </div>
    <div class="actions">
      {#if status === "downloading"}
        <span class="muted">Downloading…</span>
      {:else}
        <button class="primary" on:click={install}>Update now</button>
        <button on:click={() => (dismissed = true)}>Later</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .banner {
    position: fixed;
    top: 44px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 1rem;
    background: var(--bg-2);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 0.5rem 0.85rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
  }
  .info { display: flex; flex-direction: column; gap: 0.15rem; font-size: 12px; }
  .err { color: var(--danger); font-size: 11px; }
  .actions { display: flex; gap: 0.4rem; }
  .muted { color: var(--text-2); font-size: 12px; }
</style>
