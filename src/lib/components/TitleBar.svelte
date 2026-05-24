<script lang="ts">
  import RoutingHealthDot from "./RoutingHealthDot.svelte";
  import WindowControls from "./WindowControls.svelte";
  import ratIcon from "../icons/ratmic.png";
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";

  let version = "";
  onMount(async () => {
    try {
      version = await getVersion();
    } catch (_) {
      version = "";
    }
  });
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <img class="rat" src={ratIcon} alt="" draggable="false" />
    <span class="name">RatMic</span>
    {#if version}<span class="version">v{version}</span>{/if}
  </div>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="health" data-tauri-drag-region>
    <RoutingHealthDot />
  </div>
  <WindowControls />
</div>

<style>
  .titlebar {
    height: 36px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 1rem;
    padding-left: 0.75rem;
    user-select: none;
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    color: var(--text-0);
    flex-shrink: 0;
  }
  .rat {
    display: block;
    width: 20px;
    height: 20px;
    object-fit: contain;
    -webkit-user-drag: none;
  }
  .name {
    letter-spacing: 0.02em;
  }
  .version {
    font-size: 10px;
    color: var(--text-2);
    font-weight: 500;
    letter-spacing: 0.02em;
  }
  .spacer {
    flex: 1;
    min-width: 0;
  }
  .health {
    padding-right: 0.75rem;
    flex-shrink: 0;
  }
</style>
