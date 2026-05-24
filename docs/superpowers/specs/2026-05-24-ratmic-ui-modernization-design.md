# RatMic UI Modernization — Design Spec

**Date:** 2026-05-24
**Status:** Approved for planning
**Author:** Claude Code (with user)
**Scope:** Medium UI/UX overhaul, sleek pro-audio aesthetic. Splits into two implementation plans (6a: shell + visual foundation, 6b: chain + preset interactions).

---

## 1. Goals

- Bring RatMic up to the visual quality of modern desktop audio apps (Krisp, NVIDIA Broadcast, Discord, audio plugins) without losing the project's tone (funny but professional — the preset names carry the comedy).
- Improve scanability of the effect chain so the user can read the current state at a glance, without diving into a params panel.
- Replace the OS title bar with a custom one that matches the theme and recovers screen real estate.
- Keep all current functionality working; this is presentation + interaction, not new features.

## 2. Non-Goals

- New audio effects, new effect parameters, or new built-in presets.
- New audio engine behaviour beyond a single `reorder_effects` command.
- Hotkey, local monitor, test record (deferred to a future plan).
- Internal-SR resampling, routing health detail UI, Safe Output Mode clamps (deferred).
- Full re-imagined layout (tabs, wizards, etc.).

## 3. Aesthetic Direction

**Sleek pro-audio** — confident dark UI with a warm amber accent. Reads serious; preset names carry the personality. Small rat iconography only where it counts (window mark, about screen). No CRT/glitch effects, no skeuomorphic knobs.

## 4. Overall Layout

Two-column body, custom title bar at top, redesigned status/control bar at bottom.

```
┌──────────────────────────────────────────────────────────────────┐
│ 🐀 RatMic   [Input ▾]  [Output ▾]      ● healthy    ─  □  ✕     │ ← custom title bar
├──────────────┬───────────────────────────────────────────────────┤
│  PRESETS     │  EFFECT CHAIN                                     │
│              │                                                   │
│  [card]      │  [row] [row] [row] [+ Add Effect]                │
│  [card]      │                                                   │
│  ...         │  (accordion params expand inline per row)         │
│              │                                                   │
│  [+ Save]    │                                                   │
├──────────────┴───────────────────────────────────────────────────┤
│ IN  ▰▰▰▰▱▱ −18 dB ▽    OUT ▰▰▰▰▰▱ −9 dB ▽  LIM ● 23%   ▶ START │ ← status/control bar
└──────────────────────────────────────────────────────────────────┘
```

The right-pane parameter panel from the current UI is **removed**. Per-effect params expand inline inside each chain row.

## 5. Palette + Typography

### CSS variables

```
--bg-0: #0a0a0c      /* deepest background */
--bg-1: #16161a      /* panels */
--bg-2: #1e1e23      /* cards / rows */
--bg-3: #2a2a32      /* hover */
--border: #303038
--text-0: #f4f4f5    /* primary */
--text-1: #a1a1aa    /* secondary */
--text-2: #71717a    /* tertiary / muted */
--accent: #f59e0b    /* warm amber (refined from #d97706) */
--accent-hot: #fbbf24
--ok: #16a34a
--warn: #d97706
--danger: #dc2626
```

### Category accent colors (used for effect type icons + row stripes)

| Category | Effects | Hex |
|---|---|---|
| Level | gain | `#a1a1aa` (neutral) |
| Filter | bandpass | `#3b82f6` (blue) |
| Glitch | bitcrusher | `#eab308` (yellow) |
| Distortion | clipper | `#dc2626` (red) |
| Noise | noise | `#f59e0b` (orange) |
| Network | packetLoss | `#a855f7` (purple) |
| Dynamics | noiseGate | `#16a34a` (green) |
| Safety | limiter | `#71717a` (muted) |

### Typography

- Font: **Inter** (web-loaded), system stack fallback (`-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`)
- Base size: **14 px** (up from 13 px)
- All numeric displays (dB, Hz, ms, %) use `font-variant-numeric: tabular-nums`.
- Section labels (BUILT-IN, YOUR PRESETS): 10 px uppercase, 0.06em letter-spacing, `--text-2`.

## 6. Custom Title Bar

Replaces the OS-decorated title bar entirely.

- `tauri.conf.json` window config: `"decorations": false`.
- Full top strip is a Svelte component; the entire strip is a drag region (`data-tauri-drag-region`) **except** for interactive elements (dropdowns, buttons, status dot).
- Window controls (`─` minimize, `□` maximize/restore, `✕` close) on the right. Wired to:
  - `getCurrentWebviewWindow().minimize()`
  - `getCurrentWebviewWindow().toggleMaximize()`
  - `getCurrentWebviewWindow().close()`
- Close button gets a red hover state for accident-resistance feedback.
- 36 px tall.
- Contents from left to right:
  1. 🐀 rat SVG mark + "RatMic" wordmark (clicking opens future About modal — for now just non-interactive).
  2. Input device dropdown.
  3. Output device dropdown.
  4. Routing health indicator: small dot, color/label per heuristic (green=healthy, amber=output looks like speakers, red=input==output or no output).
  5. Window controls cluster.

Aero Snap / Snap Layouts continue to work (wry's drag region handles this on Windows 11).

## 7. Effect Chain Row (collapsed)

```
┌─────────────────────────────────────────────────────────────────┐
│ ⋮⋮  [⊕] Bandpass     300 Hz · 3.4 kHz · +0 dB     ⏻   ⌄    × │
└─────────────────────────────────────────────────────────────────┘
```

Left-to-right anatomy:

- **⋮⋮ Drag handle** — 12 px wide grip dots, visible on row hover (`opacity: 0` → `0.7` on hover). Cursor `grab` on hover, `grabbing` while dragging.
- **Type icon** — 14 px SVG, colored per category (see §5). Background tinted with 10% of the category color.
- **Effect name** — capitalized, 600 weight.
- **Inline params summary** — key 1–4 values, formatted compactly. Hidden below 600 px width via media query. Specific summaries per effect:
  - gain: `gainDb` (e.g., "+6 dB")
  - bandpass: `lowCutHz · highCutHz · midBoostDb`
  - bitcrusher: `bitDepth-bit · sampleRateHz Hz · mix %`
  - clipper: `drive · hardClip · softClip · trimDb`
  - noise: `whiteAmount · humAmount @ humHz · crackleRate`
  - packetLoss: `dropChance % · stutterChance %`
  - noiseGate: `thresholdDb · chatterAmount`
  - limiter: `ceilingDb` only
- **⏻ Pill toggle** — replaces checkbox. ~36 × 18 px, off=neutral-grey background, on=accent amber with white circle. 120 ms slide transition.
- **⌄ Expand chevron** — rotates to `⌃` when expanded. Click to toggle. Keyboard: Enter/Space on focused row.
- **× Remove** — accent-colored on hover (red). Hidden on Limiter row.

**States:**
- Default: `--bg-2` background.
- Hover: `--bg-3`.
- Expanded: 2 px left border in category color, slight elevation shadow.
- Dragging: `transform: scale(1.02)`, `box-shadow` lift, neighbours animate around it.

## 8. Effect Chain Row (expanded)

Click chevron → height transitions over 150 ms to reveal a panel beneath the collapsed row. Panel contains the same per-effect sliders that currently live in the right pane (existing `*Editor.svelte` components reused unchanged where possible, just mounted inline).

- Only one row expanded at a time. Expanding another auto-collapses the previous.
- Sliders use the new shared Slider component (see §10).
- Reset-to-default link in expanded panel header (optional micro-feature, can be cut if it bloats Plan 6b).

## 9. Limiter Row Special-Casing

The Limiter is the safety belt — visually distinct from other rows:

- Subtle striped (`linear-gradient` 2 px) background to signal "fixed".
- Drag handle replaced with a small static dot.
- × Remove button replaced with a 🔒 lock icon (no interaction).
- ⏻ Pill toggle is disabled (always on state, faded).
- Always pinned at the bottom of the chain. Drag-and-drop logic refuses to drop any row below it, and refuses to drag the limiter row itself.

## 10. Reusable Controls

### Pill toggle (replaces all checkboxes)

- Component: `src/lib/components/PillToggle.svelte`
- Props: `checked: boolean`, `disabled: boolean`, `onChange: (v: boolean) => void`
- 36 × 18 px, 120 ms slide.

### Slider (replaces existing Slider.svelte)

- Component: `src/lib/components/Slider.svelte` (refactored)
- Props: `label`, `value`, `min`, `max`, `step`, `unit`, `onChange`
- Custom-styled `<input type="range">` (track + thumb).
- Numeric value displayed to the right of label; **becomes editable on click** (turns into a small numeric input that commits on Enter/blur).
- Tooltip on thumb during drag showing the live value.
- Tabular-num for the numeric display.

### Window controls

- Component: `src/lib/components/WindowControls.svelte`
- Three buttons (min / max / close), 46 × 36 px each.
- Hover: subtle white overlay; close-button hover: red (`--danger`).

## 11. Preset Sidebar

Replaces the current single-line button list with cards.

```
┌────────────────────────────────────┐
│ Xbox 360 Lobby                ●●●●● │
│ Compressed, clipped, noisy old…    │
└────────────────────────────────────┘
```

**Card anatomy:**
- 56 px tall.
- Top row: name (semibold) + category dots (one per effect category in the preset; up to ~5 colored dots).
- Bottom row: description in `--text-2`, single line with ellipsis.
- Active preset: 2 px left border in `--accent`, `--bg-3` background.
- User preset hover: small × delete button fades in on the right.

**Sections:**
- Headers: `BUILT-IN` and `YOUR PRESETS`, collapsible via header click.
- Built-in always shows all 10; user section can be empty.

**+ Save current** button at the bottom, sticky so it stays visible while scrolling. Same accent-amber primary style as STOP/START.

## 12. Bottom Bar + Meters

```
┌───────────────────────────────────────────────────────────────────┐
│ IN  ▰▰▰▰▱▱▱▱▱▱▱▱  −18 dB ▽    OUT ▰▰▰▰▰▱▱▱▱▱▱  −9 dB ▽          │
│                                       LIM ● 23%        ▶ START   │
└───────────────────────────────────────────────────────────────────┘
```

**Segmented meter component** (`src/lib/components/Meter.svelte`):
- ~20 segments per meter.
- Non-linear dB mapping: 60% of bar dedicated to `-24 ... 0` dB so loud values are easier to read.
- Segment colors graduate: green → yellow → red as level rises.
- **Peak-hold marker (▽)**: snaps to max peak, decays back over 1 s.
- **Numeric readout** to the right of bar, tabular-num.
- **Clipping LED** flashes red briefly when output peak > -0.5 dB.

**Limiter activity badge** (`LIM ● 23%`):
- Lit color (amber) when limiter is reducing gain in the recent window.
- Percentage = fraction of the last 500 ms where `Limiter::was_active == true`.
- Worker pushes this in the existing meter event payload (extend `MeterEvent` with `limiter_activity_pct: f32`).

**START / STOP:**
- Larger button (40 px tall, ~120 px wide).
- START: accent amber background.
- STOP: outlined red, fill on hover. Distinct shape to avoid muscle-memory misclicks.
- Engine running: subtle 2 s pulse animation on the routing-health dot in the title bar.

## 13. Animations

| Element | Animation |
|---|---|
| Effect row expand/collapse | `height` 150 ms ease-out |
| Effect row drag pick-up | `transform: scale(1.02)` 120 ms |
| Effect row neighbours during drag | `transform: translateY(...)` 200 ms |
| Pill toggle | `transform` + bg-color 120 ms |
| Preset card hover | `bg` 100 ms |
| Meter bar segments | repaint on each frame; no CSS transition (sample-driven) |
| Peak-hold marker | decay 1000 ms linear |
| Clipping LED | opacity flash 200 ms |
| Routing-health dot pulse | 2 s ease-in-out infinite when engine running |

Reduced-motion preference (`prefers-reduced-motion: reduce`): pulse and expand animations skip to final state instantly. Drag-to-reorder retains motion (it's the affordance).

## 14. Drag-to-Reorder

**Library:** [`svelte-dnd-action`](https://github.com/isaacHagoel/svelte-dnd-action) v0.9+, ~3 KB gzipped, actively maintained.

**Backend** (Plan 6b):
- New IPC command: `reorder_effects(from: usize, to: usize) -> Result<(), String>`.
- New method on `EffectChain`: `move_slot(from: usize, to: usize) -> bool`. Refuses if either index is the Limiter slot or out of range. Otherwise removes from `from`, inserts at adjusted `to`.
- New method on `AudioEngine`: `reorder_effects(from, to)` that locks the chain mutex and calls `move_slot`.

**Frontend:**
- `EffectChain.svelte` wraps the list in `use:dndzone` with `dragDisabled` for the Limiter slot.
- On `consider` and `finalize` events, update local order optimistically, then call `ipc.reorderEffects(from, to)`. On error, revert.
- After drop, the row indices may have shifted — refresh via `getChain()` to resync.

## 15. Implementation Plans

### Plan 6a — Shell + Visual Foundation (smaller, ships first)

Goal: app *looks* modern. No behavioral changes beyond the title bar replacement.

Estimated tasks: ~8.

1. CSS variable refresh + Inter font load.
2. Custom title bar component + window-controls component, `decorations: false` in tauri.conf.json.
3. Routing health indicator (heuristic: dot color/text based on input/output device IDs).
4. Reusable `PillToggle` component.
5. Reusable `Slider` component (with click-to-edit numeric).
6. Refactored `Meter` component (segmented, peak-hold, dB scale).
7. Extend `MeterEvent` payload + worker push to include `limiter_activity_pct`.
8. New bottom bar layout integrating Meter + LIM badge + redesigned START/STOP.

### Plan 6b — Chain + Preset Interactions (larger, ships second)

Goal: chain and presets become scannable and editable in place.

Estimated tasks: ~10.

1. Backend: `EffectChain::move_slot` + tests.
2. Backend: `AudioEngine::reorder_effects` + `reorder_effects` Tauri command + ipc.ts wrapper.
3. Effect-type icon set (8 SVGs in `src/lib/icons/effects/`).
4. Inline parameter formatter (TS helper that returns the param-summary string per effect type).
5. New `EffectChainRow.svelte` component (drag handle, icon, summary, toggle, chevron, ×).
6. Accordion expansion logic + reuse existing `*Editor.svelte` components inline.
7. Limiter row visual special-casing + drag refusal.
8. `svelte-dnd-action` integration in `EffectChain.svelte`.
9. Preset card redesign in `PresetSidebar.svelte` (description + category dots + delete on hover + sticky save button).
10. Remove the right-pane params panel; collapse layout to two columns.

## 16. Cuttable Items (drop under time pressure)

These are nice-to-haves; cut without losing the spirit of the redesign:

- Sidebar section collapsing (built-in / user). Show both expanded always.
- Reset-to-default link in expanded effect panel header.
- Click-to-edit numeric on Slider — just show the value, no inline edit.
- Limiter activity % badge — keep the static "LIM" indicator only when active, no percentage.
- Sticky behavior on the "+ Save current" button — let it scroll normally.

## 17. Risks

| # | Risk | Mitigation |
|---|---|---|
| R1 | Custom title bar loses native conveniences (right-click menu, system-controlled snap). | Alt+Space still works for window menu. Wry handles snap. Accept the minor loss; modern apps universally accept it. |
| R2 | `svelte-dnd-action` could conflict with other event handlers. | Library is well-tested. Limit its scope to the chain list only. |
| R3 | Inline param summary may get too long on small windows. | Hide summary below 600 px via media query; user can expand the row for full info. |
| R4 | Removing the right pane breaks muscle memory. | Migration is one-time; the accordion is more efficient long-term. |
| R5 | Refactoring `EffectChain.svelte` while keeping behavior intact is non-trivial. | Plan 6b lands as one cohesive PR-shaped unit, not piecemeal. Manual smoke test verifies parity. |
| R6 | Custom title bar can break on multi-monitor / high-DPI in subtle ways. | Test on the user's actual monitor; rely on wry's drag-region implementation. |

## 18. Testing Plan

Most of this work is visual + interactive — covered by manual smoke tests after each plan. Specific items:

**Plan 6a manual tests:**
- App launches with custom title bar. Drag works. All three window controls work. Close button has red hover.
- Routing health dot reflects device choices correctly (same I/O → red, default output → amber, virtual cable name → green).
- Pill toggles replace all checkboxes; clicking toggles audio effects exactly like before.
- Meters animate smoothly with proper peak-hold decay; clipping LED flashes on loud input; LIM % updates when limiter is active.
- Reduced-motion preference skips pulse animations.

**Plan 6b manual tests + automated:**
- `EffectChain::move_slot` unit tests: valid move reorders, limiter-source refuses, limiter-target refuses, out-of-range refuses, no-op identity move.
- `cargo test` passes (all 94+ existing + ~3 new).
- Chain rows show correct inline param summaries for each of the 8 effect types.
- Drag-to-reorder works smoothly, neighbors animate, limiter cannot be dragged or dropped past.
- Expanding a chain row shows its params; expanding another auto-collapses the first.
- Removing an expanded row collapses cleanly.
- Preset cards show name + description + category dots; clicking loads; hover × deletes user presets.

## 19. File / Folder Structure Additions

New files only — existing files modified in place.

```
src/
├── lib/
│   ├── components/
│   │   ├── PillToggle.svelte           [new, 6a]
│   │   ├── Meter.svelte                [new, 6a]
│   │   ├── WindowControls.svelte       [new, 6a]
│   │   ├── TitleBar.svelte             [new, 6a]
│   │   ├── EffectChainRow.svelte       [new, 6b]
│   │   ├── RoutingHealthDot.svelte     [new, 6a]
│   │   └── effects/Slider.svelte       [refactored, 6a]
│   ├── icons/
│   │   ├── ratmic.svg                  [new, 6a — placeholder or user-provided]
│   │   └── effects/                    [new, 6b]
│   │       ├── gain.svg
│   │       ├── bandpass.svg
│   │       ├── bitcrusher.svg
│   │       ├── clipper.svg
│   │       ├── noise.svg
│   │       ├── packet-loss.svg
│   │       ├── noise-gate.svg
│   │       └── limiter.svg
│   └── format/
│       └── effect-params.ts            [new, 6b — inline summary formatter]
```

Backend changes (Plans 6a + 6b):
- `src-tauri/src/effects/chain.rs` — add `move_slot` method + tests (6b).
- `src-tauri/src/audio/engine.rs` — add `reorder_effects` method (6b).
- `src-tauri/src/audio/engine.rs` worker loop — push `limiter_activity_pct` in meter snapshot (6a).
- `src-tauri/src/audio/meters.rs` — add limiter activity tracker (6a).
- `src-tauri/src/events.rs` — extend `MeterEvent` with `limiter_activity_pct` (6a).
- `src-tauri/src/commands.rs` — `reorder_effects` command (6b).
- `src-tauri/src/lib.rs` — register new command (6b).
- `src-tauri/tauri.conf.json` — `"decorations": false` (6a).

## 20. Open Questions

None blocking. Re-evaluate during implementation:

- Should the routing-health dot be clickable to open a fuller diagnostics popover? Defer until Plan 5 (the safety/diagnostics plan).
- Should category dots on preset cards reflect only *enabled* effects in the preset, or *all* effects? Decision: only enabled, to reflect what the preset actually does on load.

---

## Approval

Approved verbally 2026-05-24 across all four sections. Proceeding to write Plan 6a via the writing-plans skill.
