export type MarkState = "idle" | "recording" | "exporting" | "directing";

export const WAVE_LAMBDA = 88;

export function flowSeconds(state: MarkState, pct = 0): number {
  if (state === "idle") return 0;
  if (state === "exporting") {
    const t = Math.max(0, Math.min(100, pct)) / 100;
    return 3 - t * (3 - 0.8);
  }
  return 2;
}

export function dotPulses(state: MarkState): boolean {
  return state === "recording";
}

export function dotTint(state: MarkState): string | null {
  return state === "directing" ? "var(--e-ai)" : null;
}
