export type EffectCategory =
  | "level" | "filter" | "glitch" | "distortion"
  | "noise" | "network" | "dynamics" | "safety";

const CATEGORY: Record<string, EffectCategory> = {
  gain: "level",
  bandpass: "filter",
  bitcrusher: "glitch",
  clipper: "distortion",
  noise: "noise",
  packetLoss: "network",
  noiseGate: "dynamics",
  limiter: "safety",
};

export function categoryOf(typeName: string): EffectCategory {
  return CATEGORY[typeName] ?? "level";
}

/** CSS custom-property reference for an effect's category color. */
export function categoryColor(typeName: string): string {
  return `var(--cat-${categoryOf(typeName)})`;
}

/** Human-friendly display name for an effect type. */
export function displayName(typeName: string): string {
  const names: Record<string, string> = {
    gain: "Gain",
    bandpass: "Bandpass",
    bitcrusher: "Bitcrusher",
    clipper: "Clipper",
    noise: "Noise",
    packetLoss: "Packet Loss",
    noiseGate: "Noise Gate",
    limiter: "Limiter",
  };
  return names[typeName] ?? typeName;
}
