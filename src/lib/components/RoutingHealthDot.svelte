<script lang="ts">
  import { inputDeviceId, outputDeviceId, engineRunning, monitorDeviceId, monitorEnabled } from "../stores";
  import { isVirtualCable } from "../format/devices";

  type Health = { color: "ok" | "warn" | "danger" | "muted"; label: string };

  function evaluate(
    input: string | null,
    output: string | null,
    monitorId: string | null,
    monEnabled: boolean,
  ): Health {
    // Feedback risk: monitor = input while listen is on
    if (monEnabled && monitorId && monitorId === input) {
      return { color: "danger", label: "feedback risk: monitor = input" };
    }
    if (!input && !output) return { color: "muted", label: "no devices" };
    if (!output) return { color: "danger", label: "no output" };
    if (input && input === output) return { color: "danger", label: "in = out (feedback)" };

    if (isVirtualCable(output)) {
      return { color: "ok", label: "routed to virtual cable" };
    }
    return { color: "warn", label: "output looks like speakers" };
  }

  $: health = evaluate($inputDeviceId, $outputDeviceId, $monitorDeviceId, $monitorEnabled);
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
