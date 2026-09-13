// True-sine geometry, shared by every wave the app draws (the recording meter, the processing
// sweep, the idle/horizon waves). One sampler so a "wave" is always literally y = A sin(kx + p) -
// never a hand-tuned bezier that only looks like one - and so the shape is testable without a DOM.

export const TAU = Math.PI * 2;

export interface SineSpec {
  /** Width the path spans, in px (x runs 0..w). */
  w: number;
  /** y of the midline, in px. SVG y grows downward, so a crest sits at `mid - amp`. */
  mid: number;
  /** Peak height above the midline, in px (half the peak-to-peak height). */
  amp: number;
  /** Spatial period, in px. */
  lambda: number;
  /** Phase, in radians - the temporal clock enters here, never as a transform. */
  phase: number;
  /** Sampling step, in px. Default 2: at 2px a 24px-tall sine is smooth at any DPI we ship. */
  step?: number;
  /** Optional 0..1 amplitude envelope, called with the normalised x (0..1). */
  envelope?: (t: number) => number;
}

/** The wave's y at `x`, in the same px space as `spec.mid`. */
export function sineY(spec: SineSpec, x: number): number {
  const env = spec.envelope ? spec.envelope(spec.w > 0 ? x / spec.w : 0) : 1;
  return spec.mid - spec.amp * env * Math.sin((TAU * x) / spec.lambda + spec.phase);
}

/** An SVG `d` for the sine, sampled every `step` px across `0..w` (the last sample always lands
 *  exactly on `w`, so the stroke reaches the right edge whatever the step divides into). */
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

/** The x of the last crest at or before `before` - where a dot "riding the crest" would sit if it
 *  hopped rather than floating. Exported for the sweep's scanning head, which parks its dot on the
 *  real crest behind it instead of on an arbitrary phase. */
export function crestBefore(spec: SineSpec, before: number): number {
  // crest <=> TAU*x/lambda + phase = PI/2 + 2*PI*k
  const k = Math.floor(((TAU * before) / spec.lambda + spec.phase - Math.PI / 2) / TAU);
  return ((Math.PI / 2 + TAU * k - spec.phase) * spec.lambda) / TAU;
}
