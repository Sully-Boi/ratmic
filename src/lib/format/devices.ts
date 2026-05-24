const VIRTUAL_KEYWORDS = ["cable", "virtual", "voicemeeter", "blackhole", "voicemod"];

export function isVirtualCable(name: string): boolean {
  const lower = name.toLowerCase();
  return VIRTUAL_KEYWORDS.some((k) => lower.includes(k));
}
