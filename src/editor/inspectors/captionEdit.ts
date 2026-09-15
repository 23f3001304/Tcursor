import type { Caption } from "../../shared/edit";

export interface SplitPoint {
  atMs: number;
  label: string;
}

export const insideSpan = (cap: Caption, ms: number) => ms > cap.start_ms && ms < cap.end_ms;

export function splitPoints(cap: Caption): SplitPoint[] {
  const out: SplitPoint[] = [];
  for (const word of cap.words.slice(1)) {
    if (!insideSpan(cap, word.start_ms)) continue;
    if (out.some((p) => p.atMs === word.start_ms)) continue;
    out.push({ atMs: word.start_ms, label: word.text });
  }
  return out;
}

export function wordsSurvive(cap: Caption, text: string): boolean {
  if (cap.words.length === 0) return false;
  return cap.words.map((w) => w.text).join(" ") === text.trim();
}

export function nextCaption(caps: Caption[], id: string): Caption | null {
  const self = caps.find((c) => c.id === id);
  if (!self) return null;
  const later = caps.filter((c) => c.id !== id && c.start_ms >= self.start_ms);
  if (later.length === 0) return null;
  return later.reduce((best, c) => (c.start_ms < best.start_ms ? c : best));
}
