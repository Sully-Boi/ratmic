<script lang="ts">
  import { onMount } from "svelte";
  import { ipc } from "../ipc";
  import { presets, chain, engineRunning } from "../stores";
  import SavePresetDialog from "./SavePresetDialog.svelte";
  import { categoryColor } from "../format/effect-meta";

  let saveOpen = false;
  let busy = false;
  let error = "";

  async function refresh() {
    try {
      const list = await ipc.listPresets();
      presets.set(list);
    } catch (e) {
      error = String(e);
    }
  }

  async function load(name: string, builtin: boolean) {
    if (!$engineRunning) {
      error = "start the engine first";
      return;
    }
    busy = true;
    error = "";
    try {
      await ipc.loadPreset(name, builtin);
      chain.set(await ipc.getChain());
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function del(name: string) {
    busy = true;
    error = "";
    try {
      await ipc.deleteUserPreset(name);
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function onSave(name: string, description: string) {
    saveOpen = false;
    busy = true;
    error = "";
    try {
      await ipc.savePresetFromChain(name, description || null);
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  $: builtIns = $presets.filter((p) => p.builtin);
  $: userPresets = $presets.filter((p) => !p.builtin);

  onMount(refresh);
</script>

<h3>Presets</h3>

<button
  class="primary save-btn"
  on:click={() => (saveOpen = true)}
  disabled={!$engineRunning || busy}
>
  + Save current as preset
</button>

{#if error}
  <p class="err">{error}</p>
{/if}

<h4>Built-in</h4>
<ul class="cards">
  {#each builtIns as p}
    <li>
      <button class="card" on:click={() => load(p.name, true)}>
        <span class="title">{p.name}</span>
        {#if p.description}<span class="desc">{p.description}</span>{/if}
        <span class="dots">
          {#each p.effect_types as t}
            <span class="dot" style="background: {categoryColor(t)}"></span>
          {/each}
        </span>
      </button>
    </li>
  {/each}
</ul>

<h4>Your presets</h4>
{#if userPresets.length === 0}
  <p class="muted">(none yet)</p>
{:else}
  <ul class="cards">
    {#each userPresets as p}
      <li class="user">
        <button class="card" on:click={() => load(p.name, false)}>
          <span class="title">{p.name}</span>
          {#if p.description}<span class="desc">{p.description}</span>{/if}
          <span class="dots">
            {#each p.effect_types as t}
              <span class="dot" style="background: {categoryColor(t)}"></span>
            {/each}
          </span>
        </button>
        <button class="del" title="Delete preset" on:click={() => del(p.name)}>×</button>
      </li>
    {/each}
  </ul>
{/if}

<SavePresetDialog
  open={saveOpen}
  onSave={onSave}
  onCancel={() => (saveOpen = false)}
/>

<style>
  h3 { margin: 0 0 0.5rem; font-size: 13px; color: var(--text-1); }
  h4 {
    margin: 0.75rem 0 0.25rem;
    font-size: 11px;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .save-btn { width: 100%; margin-bottom: 0.5rem; font-size: 12px; }
  .err { color: var(--danger); font-size: 11px; margin: 0.25rem 0; }
  .muted { color: var(--text-2); font-size: 12px; margin: 0; }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .cards li { position: relative; }
  .card {
    width: 100%;
    text-align: left;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.45rem 0.6rem;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    color: var(--text-0);
  }
  .card:hover { background: var(--bg-3); }
  .title { font-weight: 600; font-size: 12px; }
  .desc {
    font-size: 11px;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dots { display: flex; gap: 3px; margin-top: 1px; }
  .dot { width: 6px; height: 6px; border-radius: 50%; }
  .del {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 18px;
    height: 18px;
    padding: 0;
    font-size: 12px;
    line-height: 1;
    border-radius: 4px;
    background: var(--bg-3);
    border: 1px solid var(--border);
    color: var(--text-2);
    opacity: 0;
    cursor: pointer;
  }
  .user:hover .del { opacity: 1; }
  .del:hover { background: var(--danger); color: #fff; }
</style>
