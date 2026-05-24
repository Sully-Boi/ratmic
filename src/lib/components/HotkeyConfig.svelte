<script lang="ts">
  import { ipc, type HotkeyConfig, type HotkeyMode } from "../ipc";
  import { hotkey } from "../stores";

  let capturing = false;
  let error = "";

  const MODIFIER_CODES = new Set([
    "ControlLeft", "ControlRight", "AltLeft", "AltRight",
    "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight",
  ]);

  function label(cfg: HotkeyConfig | null): string {
    if (!cfg) return "none";
    const parts: string[] = [];
    if (cfg.ctrl) parts.push("Ctrl");
    if (cfg.alt) parts.push("Alt");
    if (cfg.shift) parts.push("Shift");
    parts.push(cfg.code.replace(/^Key|^Digit/, ""));
    return parts.join(" + ");
  }

  function startCapture() {
    capturing = true;
    error = "";
  }

  async function onKeydown(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    if (MODIFIER_CODES.has(e.code)) return; // wait for a non-modifier key
    const cfg: HotkeyConfig = {
      code: e.code,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      shift: e.shiftKey,
      mode: $hotkey?.mode ?? "toggle",
    };
    capturing = false;
    try {
      await ipc.setHotkey(cfg);
      hotkey.set(cfg);
    } catch (err) {
      error = String(err);
    }
  }

  async function setMode(mode: HotkeyMode) {
    if (!$hotkey) return;
    const cfg = { ...$hotkey, mode };
    try {
      await ipc.setHotkey(cfg);
      hotkey.set(cfg);
    } catch (err) {
      error = String(err);
    }
  }

  async function clear() {
    try {
      await ipc.clearHotkey();
      hotkey.set(null);
    } catch (err) {
      error = String(err);
    }
  }

  function handleModeChange(e: Event) {
    setMode((e.target as HTMLSelectElement).value as HotkeyMode);
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="hotkey">
  <span class="lbl">Hotkey</span>
  <button class="key" class:capturing on:click={startCapture}>
    {capturing ? "press a key…" : label($hotkey)}
  </button>
  {#if $hotkey}
    <select value={$hotkey.mode} on:change={handleModeChange}>
      <option value="toggle">toggle</option>
      <option value="hold">hold</option>
    </select>
    <button class="clear" title="Clear hotkey" on:click={clear}>×</button>
  {/if}
  {#if error}<span class="err">{error}</span>{/if}
</div>

<style>
  .hotkey { display: flex; align-items: center; gap: 0.4rem; font-size: 11px; color: var(--text-2); flex-shrink: 0; }
  .lbl { letter-spacing: 0.05em; font-weight: 600; }
  .key { font-size: 11px; padding: 2px 8px; min-width: 70px; }
  .key.capturing { border-color: var(--accent); color: var(--accent); }
  select { font-size: 11px; padding: 2px 4px; }
  .clear { width: 20px; padding: 0; }
  .err { color: var(--danger); }
</style>
