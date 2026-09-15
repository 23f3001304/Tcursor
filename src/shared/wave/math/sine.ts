export const TAU = Math.PI * 2;

export interface SineSpec {
  w: number;
  mid: number;
  amp: number;
  lambda: number;
  phase: number;
  step?: number;
  envelope?: (t: number) => number;
}

export function sineY(spec: SineSpec, x: number): number {
  const env = spec.envelope ? spec.envelope(spec.w > 0 ? x / spec.w : 0) : 1;
  return spec.mid - spec.amp * env * Math.sin((TAU * x) / spec.lambda + spec.phase);
}

export function sinePath(spec: SineSpec): string {
  const step = spec.step ?? 2;
  const n = Math.max(1, Math.ceil(spec.w / step));
  const parts: string[] = [];
  for (let i = 0; i <= n; i++) {
    const x = (i / n) * spec.w;
    parts.push(`${i === 0 ? "M" : "L"} ${x.toFixed(2)} ${sineY(spec, x).toFixed(2)}`);
  }
  return parts.join(" ");
}

export function crestBefore(spec: SineSpec, before: number): number {
  const k = Math.floor(((TAU * before) / spec.lambda + spec.phase - Math.PI / 2) / TAU);
  return ((Math.PI / 2 + TAU * k - spec.phase) * spec.lambda) / TAU;
}
