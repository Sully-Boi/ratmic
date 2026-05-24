<script lang="ts">
  import { onMount } from "svelte";
  import { ipc, type DeviceInfo } from "../ipc";
  import { inputDeviceId, outputDeviceId } from "../stores";
  import { isVirtualCable } from "../format/devices";
  import { openUrl } from "@tauri-apps/plugin-opener";

  export let open: boolean;
  export let onClose: () => void;

  let devices: DeviceInfo[] = [];
  let loadError = "";

  async function loadDevices() {
    loadError = "";
    try {
      devices = await ipc.listDevices();
    } catch (e) {
      loadError = String(e);
    }
  }

  onMount(loadDevices);

  $: virtualCableOutput = devices.find((d) => d.kind === "Output" && isVirtualCable(d.name));
  $: step1Done = !!virtualCableOutput;
  $: step2Done = !!$inputDeviceId && !!$outputDeviceId && isVirtualCable(
    devices.find((d) => d.id === $outputDeviceId)?.name ?? $outputDeviceId ?? ""
  );

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  async function openVbCable() {
    try {
      await openUrl("https://vb-audio.com/Cable/");
    } catch {
      // ignore — URL shown as fallback text below
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <div class="backdrop" on:click={onClose} on:keydown={handleKey} role="dialog" tabindex="-1">
    <div class="dialog" on:click|stopPropagation role="document">
      <h2>Setup RatMic</h2>
      <p class="subtitle">Follow these steps to route your mic through RatMic.</p>

      <ol class="steps">
        <!-- Step 1: Virtual cable -->
        <li class:done={step1Done}>
          <span class="status">{step1Done ? "✅" : "◯"}</span>
          <div class="step-body">
            <strong>Install a virtual audio cable</strong>
            {#if step1Done}
              <span class="detected">Detected: {virtualCableOutput?.name}</span>
            {:else}
              <span class="hint">Needed to route audio from RatMic to Discord/OBS.</span>
              <div class="step-actions">
                <button class="primary" on:click={openVbCable}>Get VB-CABLE</button>
                <button on:click={loadDevices}>Refresh</button>
              </div>
              <span class="url-fallback">https://vb-audio.com/Cable/</span>
            {/if}
          </div>
        </li>

        <!-- Step 2: Device routing -->
        <li class:done={step2Done}>
          <span class="status">{step2Done ? "✅" : "◯"}</span>
          <div class="step-body">
            <strong>Point RatMic at it</strong>
            {#if step2Done}
              <span class="detected">Input → output routed through virtual cable.</span>
            {:else}
              <span class="hint">Set <em>Input</em> to your mic and <em>Output</em> to CABLE Input in the bar above.</span>
            {/if}
          </div>
        </li>

        <!-- Step 3: Discord -->
        <li>
          <span class="status">ℹ️</span>
          <div class="step-body">
            <strong>Tell Discord to listen</strong>
            <span class="hint">In Discord (or your game/OBS), set your microphone to <strong>CABLE Output</strong>.</span>
          </div>
        </li>

        <!-- Step 4: Test -->
        <li>
          <span class="status">ℹ️</span>
          <div class="step-body">
            <strong>Test it</strong>
            <span class="hint">Pick a preset, hit <strong>START</strong>, and flip the <strong>Listen</strong> toggle to hear yourself.</span>
          </div>
        </li>
      </ol>

      {#if loadError}
        <p class="err">{loadError}</p>
      {/if}

      <div class="footer">
        <button class="primary got-it" on:click={onClose}>Got it</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 20;
  }
  .dialog {
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 1.25rem 1.5rem;
    width: 400px;
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    color: var(--text-1);
    font-weight: 700;
    letter-spacing: 0.03em;
  }
  .subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--text-2);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  li {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    background: var(--bg-0, #111);
    border: 1px solid var(--border);
    transition: border-color 150ms ease;
  }
  li.done {
    border-color: var(--ok, #4caf50);
  }
  .status {
    font-size: 14px;
    flex-shrink: 0;
    line-height: 1.4;
  }
  .step-body {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }
  strong {
    font-size: 13px;
    color: var(--text-1);
    font-weight: 600;
  }
  .hint {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1.4;
  }
  .detected {
    font-size: 12px;
    color: var(--ok, #4caf50);
  }
  .step-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }
  .url-fallback {
    font-size: 11px;
    color: var(--text-2);
    word-break: break-all;
    user-select: text;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.25rem;
  }
  .got-it {
    min-width: 100px;
    height: 34px;
    font-size: 13px;
    font-weight: 700;
  }
  .err {
    font-size: 12px;
    color: var(--danger);
    margin: 0;
  }
  /* Reuse app-level button/primary styles from global CSS */
  button.primary {
    background: var(--accent);
    color: #1a1a1a;
    border-color: var(--accent);
    font-weight: 600;
  }
  button.primary:hover:not(:disabled) {
    background: var(--accent-hot);
    border-color: var(--accent-hot);
  }
</style>
