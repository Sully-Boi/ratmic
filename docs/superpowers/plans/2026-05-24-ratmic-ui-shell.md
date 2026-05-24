# RatMic UI Shell + Visual Foundation Implementation Plan (Plan 6a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the visual foundation of the UI modernization: refreshed palette + Inter typography, custom title bar with window controls (replacing the OS-decorated title bar), a routing-health indicator, reusable pill-toggle + slider primitives, a segmented LED-style meter, and a redesigned bottom bar with limiter activity readout.

**Architecture:** Pure presentation/foundation work in the frontend, plus one backend extension to surface limiter activity via the meter event. No layout structure changes (the existing three-pane shell still renders) — those come in Plan 6b. The title bar is implemented by setting Tauri's window `decorations: false` and building a Svelte component that occupies the same vertical space.

**Tech Stack:** Tauri 2, Svelte 4 + TS, plus `@fontsource/inter` 5.x for offline-capable font bundling. No other new deps.

**User preference:** No git unless asked.

**Spec:** `docs/superpowers/specs/2026-05-24-ratmic-ui-modernization-design.md`. Tasks below implement Plan 6a in §15 of the spec.

---

## File Structure Map

| Task | File |
|---|---|
| 1 | `src/app.css` (palette + typography rewrite), `package.json` (+ `@fontsource/inter`), `src/main.ts` (font import) |
| 2 | `src/lib/components/PillToggle.svelte` (new) |
| 2 | `src/lib/components/effects/*Editor.svelte` (replace `<input type=checkbox>` where applicable — Limiter editor doesn't have one currently; EffectChain.svelte does) |
| 2 | `src/lib/components/EffectChain.svelte` (use PillToggle) |
| 3 | `src/lib/components/effects/Slider.svelte` (refactor: click-to-edit value, drag tooltip) |
| 4 | `src/lib/components/WindowControls.svelte` (new) |
| 5 | `src/lib/components/RoutingHealthDot.svelte` (new) |
| 6 | `src/lib/components/TitleBar.svelte` (new) |
| 6 | `src/lib/icons/ratmic.svg` (placeholder rat mark) |
| 7 | `src-tauri/tauri.conf.json` (`decorations: false`), `src/App.svelte` (mount TitleBar, remove old DeviceBar slot from header) |
| 8 | `src-tauri/src/effects/mod.rs` (`Effect` trait extension), `src-tauri/src/effects/limiter.rs` (override), `src-tauri/src/effects/chain.rs` (`limiter_was_active`), `src-tauri/src/events.rs` (extend `MeterEvent`), `src-tauri/src/audio/engine.rs` (worker tracker + emit) |
| 8 | `src/lib/ipc.ts` (extend `MeterEvent` TS type) |
| 9 | `src/lib/components/Meter.svelte` (segmented, peak-hold, dB scale, clipping LED) |
| 10 | `src/App.svelte` (bottom-bar layout + START/STOP styling), `src/lib/components/MeterBar.svelte` (replaced by `Meter.svelte` callers) |

---

## Task 1: Palette + typography refresh

**Files:**
- Modify: `E:\ClaudeCode\ratmic\package.json` (add `@fontsource/inter`)
- Modify: `E:\ClaudeCode\ratmic\src\main.ts` (import Inter)
- Modify: `E:\ClaudeCode\ratmic\src\app.css` (palette overhaul)

- [ ] **Step 1: Install Inter font package**

Run: `npm install @fontsource/inter`
Expected: package added, ~600 KB installed (multiple weights + variable font).

- [ ] **Step 2: Import Inter weights from `main.ts`**

Replace `src/main.ts`:

```ts
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "./app.css";
import App from "./App.svelte";

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
```

- [ ] **Step 3: Replace `src/app.css` with refreshed palette**

Replace the entire file with:

```css
:root {
  /* Backgrounds */
  --bg-0: #0a0a0c;
  --bg-1: #16161a;
  --bg-2: #1e1e23;
  --bg-3: #2a2a32;
  --border: #303038;

  /* Text */
  --text-0: #f4f4f5;
  --text-1: #a1a1aa;
  --text-2: #71717a;

  /* Accent + status */
  --accent: #f59e0b;
  --accent-hot: #fbbf24;
  --ok: #16a34a;
  --warn: #d97706;
  --danger: #dc2626;

  /* Category accents (effect rows + preset dots) */
  --cat-level: #a1a1aa;
  --cat-filter: #3b82f6;
  --cat-glitch: #eab308;
  --cat-distortion: #dc2626;
  --cat-noise: #f59e0b;
  --cat-network: #a855f7;
  --cat-dynamics: #16a34a;
  --cat-safety: #71717a;

  /* Numeric type */
  font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  font-size: 14px;
  font-feature-settings: "cv11", "ss03";
  color-scheme: dark;
}

* {
  box-sizing: border-box;
}

html, body, #app {
  height: 100%;
  margin: 0;
}

body {
  background: var(--bg-0);
  color: var(--text-0);
  overflow: hidden;
  user-select: none;
}

button {
  background: var(--bg-2);
  color: var(--text-0);
  border: 1px solid var(--border);
  padding: 0.45rem 0.85rem;
  border-radius: 6px;
  font: inherit;
  font-weight: 500;
  cursor: pointer;
  transition: background 100ms ease, border-color 100ms ease;
}

button:hover:not(:disabled) {
  background: var(--bg-3);
  border-color: #404048;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

button.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #1a1a1a;
}

button.primary:hover:not(:disabled) {
  background: var(--accent-hot);
  border-color: var(--accent-hot);
}

select, input[type="text"], input[type="number"] {
  background: var(--bg-1);
  color: var(--text-0);
  border: 1px solid var(--border);
  padding: 0.35rem 0.55rem;
  border-radius: 6px;
  font: inherit;
}

select:focus, input:focus {
  outline: none;
  border-color: var(--accent);
}

.tabular {
  font-variant-numeric: tabular-nums;
}

@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

- [ ] **Step 4: Verify TypeScript still passes**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 5: Visual sanity check (deferred to human)**

The app should now use Inter font and the new color palette. No structural changes yet — same three-pane layout, but with the refreshed look. Human can verify after Plan 6a's final smoke test.

---

## Task 2: PillToggle primitive

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\PillToggle.svelte`
- Modify: `E:\ClaudeCode\ratmic\src\lib\components\EffectChain.svelte` (replace `<input type=checkbox>` with PillToggle)

- [ ] **Step 1: Create the PillToggle component**

Create `src/lib/components/PillToggle.svelte`:

```svelte
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
```

- [ ] **Step 2: Replace the checkbox in EffectChain.svelte**

Open `src/lib/components/EffectChain.svelte`. Find the `<input type="checkbox">` line in the row template and replace with `<PillToggle>`. Add the import at the top of the script block.

In the script section, add:

```ts
import PillToggle from "./PillToggle.svelte";
```

In the template, replace:

```svelte
<input
  type="checkbox"
  checked={slot.enabled}
  on:change={(e) => toggle(slot.index, (e.target as HTMLInputElement).checked)}
  disabled={!$engineRunning || slot.type_name === "limiter"}
  on:click|stopPropagation
/>
```

with:

```svelte
<PillToggle
  checked={slot.enabled}
  disabled={!$engineRunning || slot.type_name === "limiter"}
  onChange={(v) => toggle(slot.index, v)}
  ariaLabel={`enable ${slot.type_name}`}
/>
```

- [ ] **Step 3: Verify TS check + visual**

Run: `npm run check`
Expected: 0 errors.

---

## Task 3: Slider refactor (click-to-edit + drag tooltip)

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\lib\components\effects\Slider.svelte`

- [ ] **Step 1: Refactor Slider with click-to-edit value + drag tooltip**

Replace `src/lib/components/effects/Slider.svelte`:

```svelte
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
```

- [ ] **Step 2: Verify TS check**

Run: `npm run check`
Expected: 0 errors.

---

## Task 4: WindowControls component

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\WindowControls.svelte`

- [ ] **Step 1: Implement WindowControls**

Create `src/lib/components/WindowControls.svelte`:

```svelte
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
```

- [ ] **Step 2: Verify capabilities allow window controls**

Open `src-tauri/capabilities/default.json` — confirm it includes `core:default` (which grants `core:window:allow-*`).

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "...",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

If `core:default` is already there, no changes needed.

- [ ] **Step 3: Verify TS check**

Run: `npm run check`
Expected: 0 errors.

---

## Task 5: RoutingHealthDot component

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\RoutingHealthDot.svelte`

- [ ] **Step 1: Implement the routing health heuristic + dot UI**

Create `src/lib/components/RoutingHealthDot.svelte`:

```svelte
<script lang="ts">
  import { inputDeviceId, outputDeviceId, engineRunning } from "../stores";

  type Health = { color: "ok" | "warn" | "danger" | "muted"; label: string };

  const VIRTUAL_KEYWORDS = ["cable", "virtual", "voicemeeter", "blackhole", "voicemod"];

  function evaluate(input: string | null, output: string | null): Health {
    if (!input && !output) return { color: "muted", label: "no devices" };
    if (!output) return { color: "danger", label: "no output" };
    if (input && input === output) return { color: "danger", label: "in = out (feedback)" };

    const lower = output.toLowerCase();
    if (VIRTUAL_KEYWORDS.some((k) => lower.includes(k))) {
      return { color: "ok", label: "routed to virtual cable" };
    }
    return { color: "warn", label: "output looks like speakers" };
  }

  $: health = evaluate($inputDeviceId, $outputDeviceId);
</script>

<div class="health" title={health.label}>
  <span class="dot" class:ok={health.color === "ok"} class:warn={health.color === "warn"} class:danger={health.color === "danger"} class:muted={health.color === "muted"} class:pulse={$engineRunning} />
  <span class="label">{health.label}</span>
</div>

<style>
  .health {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-2);
    font-size: 11px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-2);
    flex-shrink: 0;
  }
  .dot.ok { background: var(--ok); }
  .dot.warn { background: var(--warn); }
  .dot.danger { background: var(--danger); }
  .dot.muted { background: var(--text-2); }
  .dot.pulse {
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.6; transform: scale(0.85); }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.pulse { animation: none; }
  }
  .label {
    white-space: nowrap;
  }
</style>
```

- [ ] **Step 2: Verify TS check**

Run: `npm run check`
Expected: 0 errors.

---

## Task 6: TitleBar component + placeholder rat icon

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\icons\ratmic.svg`
- Create: `E:\ClaudeCode\ratmic\src\lib\components\TitleBar.svelte`

- [ ] **Step 1: Create a placeholder rat SVG**

Create `src/lib/icons/ratmic.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 20 20" fill="none">
  <!-- Stylized rat head silhouette: two ears and a nose. -->
  <circle cx="5" cy="6" r="2.5" fill="currentColor" opacity="0.85" />
  <circle cx="15" cy="6" r="2.5" fill="currentColor" opacity="0.85" />
  <path d="M3 11 Q10 18 17 11 Q14 15 10 15 Q6 15 3 11 Z" fill="currentColor" />
  <circle cx="10" cy="14.5" r="0.8" fill="#1a1a1a" />
</svg>
```

(User can swap this for their own logo via `npm run tauri icon ...` for the app icon and by replacing this file for the in-app mark.)

- [ ] **Step 2: Implement TitleBar**

Create `src/lib/components/TitleBar.svelte`:

```svelte
<script lang="ts">
  import DeviceBar from "./DeviceBar.svelte";
  import RoutingHealthDot from "./RoutingHealthDot.svelte";
  import WindowControls from "./WindowControls.svelte";
  import ratIcon from "../icons/ratmic.svg?raw";
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="rat">{@html ratIcon}</span>
    <span class="name">RatMic</span>
  </div>
  <div class="devices">
    <DeviceBar />
  </div>
  <div class="health">
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
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    color: var(--text-0);
  }
  .rat {
    display: inline-flex;
    color: var(--accent);
    width: 20px;
    height: 20px;
  }
  .rat :global(svg) {
    width: 100%;
    height: 100%;
  }
  .name {
    letter-spacing: 0.02em;
  }
  .devices {
    flex: 1;
    display: flex;
    align-items: center;
  }
  .health {
    padding-right: 0.75rem;
  }
</style>
```

- [ ] **Step 3: Verify TS + Vite handle SVG-as-raw**

Vite supports `?raw` imports out of the box for any file extension. Run: `npm run check`
Expected: 0 errors.

If a TS error appears about `?raw` import declarations, create `src/vite-env.d.ts` with:

```ts
/// <reference types="svelte" />

declare module "*.svg?raw" {
  const content: string;
  export default content;
}
```

- [ ] **Step 4: Confirm DeviceBar styles still work in narrower context**

The DeviceBar previously sat as its own header element. It will now render inside the titlebar's `.devices` flex container. Open `src/lib/components/DeviceBar.svelte` and verify the existing component fits — its `.bar` container uses `display: flex; gap: 1rem; align-items: center`, which should compose fine inside another flex parent.

If the device labels wrap awkwardly in a tight title bar, this is acceptable for now — Plan 6b will polish it further.

---

## Task 7: Enable custom title bar (decorations off) + mount in App.svelte

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\tauri.conf.json` (`decorations: false`)
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte` (replace old top-bar header with `<TitleBar>`)

- [ ] **Step 1: Disable OS decorations**

Open `src-tauri/tauri.conf.json`. In the window config, add `"decorations": false`. The relevant window object should look like:

```json
{
  "title": "RatMic",
  "width": 1100,
  "height": 720,
  "minWidth": 900,
  "minHeight": 600,
  "resizable": true,
  "fullscreen": false,
  "decorations": false
}
```

- [ ] **Step 2: Replace the existing top bar in App.svelte**

Open `src/App.svelte`. Add the TitleBar import at the top of the script block:

```ts
import TitleBar from "./lib/components/TitleBar.svelte";
```

Locate the existing top-bar markup, which currently reads roughly:

```svelte
<header class="top-bar">
  <strong>RatMic</strong>
  <DeviceBar />
</header>
```

Replace it with:

```svelte
<TitleBar />
```

Remove the now-unused `DeviceBar` import line from `App.svelte` (it lives inside `TitleBar` now).

If there's a `.top-bar` style block left in the `<style>` section of App.svelte, remove it. The grid template row that allocated 44 px for `.top-bar` should be replaced with `36px` for the new title bar:

```css
.shell { display: grid; grid-template-rows: 36px 1fr 64px; height: 100%; }
```

- [ ] **Step 3: Run the dev server briefly to verify the title bar replaces the OS-decorated one**

Manual verification — launch via `npm run tauri dev`. Expected:
- Native Windows title bar is gone.
- The custom strip shows: rat icon + "RatMic" wordmark on the left, device dropdowns center, routing-health dot, and three window-control buttons on the right.
- Dragging anywhere on the strip (except interactive controls) moves the window.
- Minimize, maximize, restore, close all work.

If the window has lost its rounded corners under Windows 11, that's a known cost of `decorations: false` and acceptable for v1.

---

## Task 8: Backend limiter activity tracking + extend MeterEvent

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\mod.rs` (add default trait method)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\limiter.rs` (override)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\effects\chain.rs` (chain helper)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\events.rs` (extend MeterEvent)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\audio\engine.rs` (worker tracker)
- Modify: `E:\ClaudeCode\ratmic\src-tauri\src\commands.rs` (EmitSink pushes new field)
- Modify: `E:\ClaudeCode\ratmic\src\lib\ipc.ts` (extend TS type)

- [ ] **Step 1: Extend `Effect` trait with a default `limiter_activity` accessor**

Open `src-tauri/src/effects/mod.rs`. Add to the `Effect` trait:

```rust
pub trait Effect: Send {
    fn type_name(&self) -> &'static str;
    fn process(&mut self, buffer: &mut [f32]);
    fn set_params(&mut self, params: &serde_json::Value) -> anyhow::Result<()>;
    fn get_params(&self) -> serde_json::Value;
    fn reset(&mut self);

    /// For the Limiter, returns whether the limiter was actively reducing gain
    /// in the most recent process() call. Returns `None` for non-limiter effects.
    fn limiter_was_active(&self) -> Option<bool> {
        None
    }
}
```

- [ ] **Step 2: Override `limiter_was_active` in Limiter**

Open `src-tauri/src/effects/limiter.rs`. In the existing `impl Effect for Limiter` block, add:

```rust
    fn limiter_was_active(&self) -> Option<bool> {
        Some(self.was_active)
    }
```

- [ ] **Step 3: Write a test that the trait override works**

Append to `src-tauri/src/effects/limiter.rs::tests`:

```rust
    #[test]
    fn limiter_reports_activity_via_trait() {
        use super::super::Effect;
        let mut l = Limiter::new(48000, LimiterParams { ceiling_db: -3.0, release_ms: 80.0 });
        // Quiet signal — not active.
        let mut buf = vec![0.1; 256];
        l.process(&mut buf);
        let trait_obj: &dyn Effect = &l;
        assert_eq!(trait_obj.limiter_was_active(), Some(false));
        // Loud signal — active.
        let mut buf = vec![0.95; 256];
        l.process(&mut buf);
        let trait_obj: &dyn Effect = &l;
        assert_eq!(trait_obj.limiter_was_active(), Some(true));
    }
```

Run: `cd src-tauri && cargo test effects::limiter`
Expected: 6 tests pass (5 prior + 1 new).

- [ ] **Step 4: Add `EffectChain::limiter_was_active`**

Open `src-tauri/src/effects/chain.rs`. Add inside `impl EffectChain`:

```rust
    /// Returns true if the chain's Limiter was actively reducing gain in the
    /// most recent process() call. Returns false if no Limiter slot is present.
    pub fn limiter_was_active(&self) -> bool {
        self.slots
            .iter()
            .filter_map(|s| s.effect.limiter_was_active())
            .next()
            .unwrap_or(false)
    }
```

Append a test:

```rust
    #[test]
    fn chain_reports_limiter_activity() {
        let mut c = EffectChain::new(48000);
        c.rebuild_from_slots(48000, vec![]);
        // Quiet input: no activity.
        let mut buf = vec![0.1; 256];
        c.process(&mut buf);
        assert!(!c.limiter_was_active());
        // Loud input: limiter kicks in.
        let mut buf = vec![0.95; 256];
        c.process(&mut buf);
        assert!(c.limiter_was_active());
    }
```

Run: `cd src-tauri && cargo test effects::chain`
Expected: 10 tests pass (9 prior + 1 new).

- [ ] **Step 5: Extend `MeterEvent` with limiter activity field**

Open `src-tauri/src/events.rs`. Add a field to `MeterEvent`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MeterEvent {
    pub input_peak_db: f32,
    pub input_rms_db: f32,
    pub output_peak_db: f32,
    pub output_rms_db: f32,
    pub limiter_activity_pct: f32,
}
```

- [ ] **Step 6: Add `limiter_activity_pct` to `MeterSnapshot` and track in worker**

Open `src-tauri/src/audio/engine.rs`. Update the snapshot struct:

```rust
#[derive(Debug, Clone, Copy)]
pub struct MeterSnapshot {
    pub input: MeterValue,
    pub output: MeterValue,
    pub limiter_activity_pct: f32,
}
```

In `worker_loop`, add a ring buffer to track recent limiter activity per processing chunk. Replace the entire `worker_loop` function body:

```rust
fn worker_loop<S: MeterSink>(
    mut consumer: RingConsumer,
    backend: Arc<Mutex<Box<dyn AudioOutputBackend>>>,
    chain: Arc<Mutex<EffectChain>>,
    sink: S,
    stop: Arc<AtomicBool>,
) {
    let mut buffer = vec![0.0_f32; WORKER_CHUNK_SAMPLES];
    let mut in_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let mut out_meter = Meter::new(INTERNAL_SAMPLE_RATE, 150.0);
    let meter_interval = Duration::from_millis(METER_TICK_MS);
    let mut last_meter = std::time::Instant::now();

    // Track limiter activity over the last ~500 ms (50 chunks of 10 ms each).
    const ACTIVITY_HISTORY: usize = 50;
    let mut activity_history = [false; ACTIVITY_HISTORY];
    let mut activity_write = 0_usize;

    while !stop.load(Ordering::Relaxed) {
        let n = consumer.pop(&mut buffer);
        if n == 0 {
            thread::sleep(Duration::from_millis(2));
            continue;
        }
        let chunk = &mut buffer[..n];
        in_meter.process(chunk);

        let mut chain_guard = chain.lock();
        chain_guard.process(chunk);
        let was_active = chain_guard.limiter_was_active();
        drop(chain_guard);

        activity_history[activity_write] = was_active;
        activity_write = (activity_write + 1) % ACTIVITY_HISTORY;

        out_meter.process(chunk);
        let _ = backend.lock().write(chunk);

        if last_meter.elapsed() >= meter_interval {
            let activity_count = activity_history.iter().filter(|x| **x).count();
            let limiter_activity_pct =
                (activity_count as f32) / (ACTIVITY_HISTORY as f32) * 100.0;

            sink.push(MeterSnapshot {
                input: in_meter.snapshot(),
                output: out_meter.snapshot(),
                limiter_activity_pct,
            });
            last_meter = std::time::Instant::now();
        }
    }
}
```

- [ ] **Step 7: Update `EmitSink` to emit the new field**

Open `src-tauri/src/commands.rs`. In `impl MeterSink for EmitSink`:

```rust
impl MeterSink for EmitSink {
    fn push(&self, snap: MeterSnapshot) {
        let ev = MeterEvent {
            input_peak_db: snap.input.peak_db(),
            input_rms_db: snap.input.rms_db(),
            output_peak_db: snap.output.peak_db(),
            output_rms_db: snap.output.rms_db(),
            limiter_activity_pct: snap.limiter_activity_pct,
        };
        let _ = self.app.emit(EVENT_METERS, ev);
    }
}
```

- [ ] **Step 8: Run the full backend test suite**

Run: `cd src-tauri && cargo test`
Expected: 94 prior + 1 limiter test + 1 chain test = **96 tests pass**.

- [ ] **Step 9: Extend the TS `MeterEvent` interface**

Open `src/lib/ipc.ts`. Update `MeterEvent`:

```ts
export interface MeterEvent {
  input_peak_db: number;
  input_rms_db: number;
  output_peak_db: number;
  output_rms_db: number;
  limiter_activity_pct: number;
}
```

Update `src/lib/stores.ts` default value of the `meters` store:

```ts
export const meters = writable<MeterEvent>({
  input_peak_db: -90,
  input_rms_db: -90,
  output_peak_db: -90,
  output_rms_db: -90,
  limiter_activity_pct: 0,
});
```

Run: `npm run check`
Expected: 0 errors.

---

## Task 9: Meter component (segmented + peak hold + clipping LED)

**Files:**
- Create: `E:\ClaudeCode\ratmic\src\lib\components\Meter.svelte`

This component replaces `MeterBar.svelte` in usage (we keep the old file around until Task 10 swaps it out completely).

- [ ] **Step 1: Implement segmented meter with peak-hold**

Create `src/lib/components/Meter.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  export let label: string;
  export let peakDb: number = -90;
  export let rmsDb: number = -90;

  const SEGMENTS = 20;
  /**
   * Map dB to a normalized [0, 1] segment fill where the loud half (-24 to 0 dB)
   * occupies ~60% of the bar.
   */
  function dbToNorm(db: number): number {
    if (db <= -60) return 0;
    if (db >= 0) return 1;
    if (db >= -24) {
      // -24 to 0 → 0.4 to 1.0
      return 0.4 + ((db + 24) / 24) * 0.6;
    } else {
      // -60 to -24 → 0 to 0.4
      return ((db + 60) / 36) * 0.4;
    }
  }

  let peakHold = -90;
  let peakHoldDecayTimer: number | null = null;

  $: if (peakDb > peakHold) {
    peakHold = peakDb;
  }

  function startDecayLoop() {
    const tick = () => {
      // Decay 60 dB over 1000 ms = 60 dB/sec.
      if (peakHold > peakDb) {
        peakHold = Math.max(peakDb, peakHold - 60 / 60); // ~1 dB per frame at 60 fps
      }
      peakHoldDecayTimer = requestAnimationFrame(tick);
    };
    peakHoldDecayTimer = requestAnimationFrame(tick);
  }

  onMount(startDecayLoop);
  onDestroy(() => {
    if (peakHoldDecayTimer !== null) cancelAnimationFrame(peakHoldDecayTimer);
  });

  $: rmsNorm = dbToNorm(rmsDb);
  $: peakHoldNorm = dbToNorm(peakHold);
  $: clipping = peakDb > -0.5;

  function segmentColor(idx: number): string {
    const t = idx / SEGMENTS;
    if (t < 0.7) return "var(--ok)";
    if (t < 0.9) return "var(--warn)";
    return "var(--danger)";
  }
</script>

<div class="meter">
  <span class="label">{label}</span>
  <div class="bar">
    {#each Array(SEGMENTS) as _, i}
      {@const filled = i / SEGMENTS < rmsNorm}
      {@const isHold = Math.abs(i / SEGMENTS - peakHoldNorm) < 1 / SEGMENTS}
      <div
        class="seg"
        class:filled
        class:hold={isHold && !filled}
        style="background: {filled ? segmentColor(i) : 'var(--bg-2)'}"
      />
    {/each}
  </div>
  <span class="value tabular" class:clipping>{peakDb.toFixed(1)} dB</span>
  <span class="led" class:on={clipping} aria-hidden="true" />
</div>

<style>
  .meter {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 11px;
    min-width: 280px;
  }
  .label {
    color: var(--text-1);
    width: 26px;
    font-weight: 600;
    letter-spacing: 0.05em;
  }
  .bar {
    flex: 1;
    height: 12px;
    display: flex;
    gap: 1px;
  }
  .seg {
    flex: 1;
    border-radius: 1px;
    transition: background 30ms linear;
  }
  .seg.hold {
    background: var(--text-0) !important;
  }
  .value {
    color: var(--text-1);
    width: 58px;
    text-align: right;
    font-weight: 600;
  }
  .value.clipping {
    color: var(--danger);
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgba(220, 38, 38, 0.2);
    flex-shrink: 0;
    transition: background 80ms ease, box-shadow 80ms ease;
  }
  .led.on {
    background: var(--danger);
    box-shadow: 0 0 6px var(--danger);
  }
</style>
```

- [ ] **Step 2: Verify TS check**

Run: `npm run check`
Expected: 0 errors.

---

## Task 10: Bottom bar redesign

**Files:**
- Modify: `E:\ClaudeCode\ratmic\src\App.svelte`

Pulls everything together: replaces the old MeterBar with the new Meter, adds the LIM activity badge, restyles the START/STOP button.

- [ ] **Step 1: Update imports in App.svelte**

In `src/App.svelte` script block, replace the line `import MeterBar from "./lib/components/MeterBar.svelte";` with:

```ts
import Meter from "./lib/components/Meter.svelte";
```

- [ ] **Step 2: Replace the bottom-bar markup**

Find the existing `<footer class="bottom-bar">` block and replace it with:

```svelte
<footer class="bottom-bar">
  <Meter label="IN"  peakDb={$meters.input_peak_db}  rmsDb={$meters.input_rms_db} />
  <Meter label="OUT" peakDb={$meters.output_peak_db} rmsDb={$meters.output_rms_db} />
  <div class="lim" class:active={$meters.limiter_activity_pct > 0}>
    <span class="dot" />
    <span>LIM</span>
    <span class="pct tabular">{Math.round($meters.limiter_activity_pct)}%</span>
  </div>
  <div class="spacer"></div>
  {#if $engineError}<span class="err">{$engineError}</span>{/if}
  <button
    class="engine-btn"
    class:running={$engineRunning}
    on:click={toggleEngine}
  >
    {$engineRunning ? "■ STOP" : "▶ START"}
  </button>
</footer>
```

- [ ] **Step 3: Update the bottom-bar styles in App.svelte**

In the `<style>` block, replace the existing `.bottom-bar` rule and add the new ones:

```css
.bottom-bar {
  display: flex;
  align-items: center;
  gap: 1.25rem;
  padding: 0 1rem;
  background: var(--bg-1);
  border-top: 1px solid var(--border);
  height: 64px;
}
.spacer { flex: 1; }
.err {
  color: var(--danger);
  font-size: 12px;
  margin-right: 0.5rem;
}
.lim {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-2);
  letter-spacing: 0.05em;
}
.lim .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-2);
  transition: background 80ms ease, box-shadow 80ms ease;
}
.lim.active { color: var(--accent); }
.lim.active .dot {
  background: var(--accent);
  box-shadow: 0 0 6px var(--accent);
}
.lim .pct {
  min-width: 32px;
  text-align: right;
  color: var(--text-1);
}
.engine-btn {
  min-width: 120px;
  height: 40px;
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.05em;
  background: var(--accent);
  color: #1a1a1a;
  border-color: var(--accent);
}
.engine-btn:hover:not(:disabled) {
  background: var(--accent-hot);
  border-color: var(--accent-hot);
}
.engine-btn.running {
  background: transparent;
  color: var(--danger);
  border-color: var(--danger);
}
.engine-btn.running:hover:not(:disabled) {
  background: var(--danger);
  color: #fff;
}
```

- [ ] **Step 4: Verify TS check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 5: Final cargo + npm verification**

Run: `cd src-tauri && cargo test`
Expected: 96/96 tests pass.

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 6: Smoke test (manual, human)**

Launch `npm run tauri dev`. Verify:

1. **Title bar**: custom strip with rat icon + "RatMic" + device dropdowns + routing health dot + min/max/close. Native OS title bar is gone. Dragging the strip moves the window. Close button has red hover.
2. **Palette**: app uses Inter font, the new warm-amber accent, refined dark backgrounds.
3. **Effect chain rows**: pill toggle replaces the checkbox. Sliders in expanded params (for now still in right pane — accordion comes in Plan 6b) have click-to-edit values and drag tooltips.
4. **Bottom bar**: segmented meters with peak-hold (small white tick that decays), clipping LED to the right of the dB value, LIM badge showing % when limiter is active. START/STOP button has new styling — START is amber primary, STOP is red outline.
5. **Functional regression check**: device selection persists, presets still load, effects still toggle audibly, all behaviors from Plan 5 still work.

---

## Final verification

When all 10 tasks are checked off:

- [ ] App launches with the new custom title bar (no OS chrome).
- [ ] Inter font + new palette visibly in effect.
- [ ] Pill toggles work everywhere the old checkboxes did.
- [ ] Sliders show drag tooltip + click-to-edit numeric value.
- [ ] Segmented meters with peak-hold animation, clipping LED, LIM activity %.
- [ ] START/STOP button distinct styling.
- [ ] All 96 Rust tests pass.
- [ ] `npm run check` clean.
- [ ] No audio/preset regressions from prior plans.

Plan 6b (chain + preset interactions: drag-to-reorder, type icons, accordion params, preset cards) is the next plan to write.
