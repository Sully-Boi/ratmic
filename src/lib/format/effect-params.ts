/** Format the key parameters of an effect into a compact one-line summary
 *  for display on a collapsed chain row. */
export function summarizeParams(typeName: string, params: unknown): string {
  const p = (params ?? {}) as Record<string, number | string | undefined>;
  const n = (v: unknown, d = 0): number =>
    typeof v === "number" ? v : d;
  const db = (v: number) => `${v >= 0 ? "+" : ""}${v.toFixed(0)} dB`;
  const pct = (v: number) => `${Math.round(v * 100)}%`;

  switch (typeName) {
    case "gain":
      return db(n(p.gainDb));
    case "bandpass":
      return `${n(p.lowCutHz, 100)} Hz · ${n(p.highCutHz, 8000)} Hz${
        n(p.midBoostDb) !== 0 ? ` · ${db(n(p.midBoostDb))}` : ""
      }`;
    case "bitcrusher":
      return `${n(p.bitDepth, 16)}-bit · ${n(p.sampleRateHz, 48000)} Hz · ${pct(n(p.mix, 1))}`;
    case "clipper":
      return `drive ${n(p.drive, 1).toFixed(1)} · hard ${n(p.hardClip, 1).toFixed(2)}`;
    case "noise": {
      const parts: string[] = [];
      if (n(p.whiteAmount) > 0) parts.push(`white ${pct(n(p.whiteAmount))}`);
      if (n(p.humAmount) > 0) parts.push(`hum ${n(p.humHz, 60)} Hz`);
      if (n(p.crackleRate) > 0) parts.push(`crackle ${n(p.crackleRate)}/s`);
      return parts.length ? parts.join(" · ") : "silent";
    }
    case "packetLoss":
      return `drop ${pct(n(p.dropChance))}${
        n(p.stutterChance) > 0 ? ` · stutter ${pct(n(p.stutterChance))}` : ""
      }`;
    case "noiseGate":
      return `${n(p.thresholdDb, -40).toFixed(0)} dB${
        n(p.chatterAmount) > 0 ? ` · chatter ${pct(n(p.chatterAmount))}` : ""
      }`;
    case "limiter":
      return `ceiling ${n(p.ceilingDb, -3).toFixed(0)} dB`;
    default:
      return "";
  }
}
