<script lang="ts">
  export let open: boolean;
  export let onSave: (name: string, description: string) => void;
  export let onCancel: () => void;

  let name = "";
  let description = "";

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter" && name.trim()) onSave(name.trim(), description.trim());
  }

  $: if (open) {
    name = "";
    description = "";
  }
</script>

{#if open}
  <div class="backdrop" on:click={onCancel} on:keydown={handleKey} role="dialog" tabindex="-1">
    <div class="dialog" on:click|stopPropagation role="document">
      <h3>Save as preset</h3>
      <label>
        Name
        <input type="text" bind:value={name} placeholder="My Cursed Mic" autofocus />
      </label>
      <label>
        Description (optional)
        <input type="text" bind:value={description} placeholder="One-line description" />
      </label>
      <div class="row">
        <button on:click={onCancel}>Cancel</button>
        <button
          class="primary"
          disabled={!name.trim()}
          on:click={() => onSave(name.trim(), description.trim())}
        >
          Save
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: grid;
    place-items: center;
    z-index: 10;
  }
  .dialog {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  h3 { margin: 0 0 0.25rem; font-size: 13px; color: var(--text-1); }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 12px;
    color: var(--text-1);
  }
  input[type="text"] { width: 100%; }
  .row {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
</style>
