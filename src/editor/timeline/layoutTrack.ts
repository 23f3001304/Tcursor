import type { LayoutSeg } from "../../lib/edit";
import type { LayoutPresets, LayoutPresetName, LayoutPresetDto, PanelRectDto, PreviewLayout } from "../../lib/ipc";
import { evalCubic, parseCubic } from "../../lib/cubicBezier";
import { spring, springOf } from "../../lib/spring";

/** One resolved panel pair - screen + cam - either a segment's own per-segment override (a posed
 *  T34 arrangement, resolved once in Rust through the exact path the export uses) or its preset's
 *  own panels. This is the ONLY place `layoutAt` reads pose-derived rects; picking which
 *  already-resolved pair to show is presentation, not pose math - the project rule (core logic
 *  once, in Rust) holds even here. */
export interface ResolvedPanels { screen: PanelRectDto; cam: PanelRectDto }

/** Easing mirror of the export's curves (crate::export::camera::ease): linear / smoothstep /
 *  quadratic ease-in / ease-out / ease-in-out, plus the two parameterised forms - a custom
 *  `cubic(x1,y1,x2,y2)` curve (crate::export::cubic) and a real damped-oscillator
 *  `spring(stiffness,damping[,mass])` (crate::export::spring), whose bare word `spring` means
 *  `SPRING_DEFAULT`. The ONLY place easing is evaluated in TS - exported so other per-frame
 *  mirrors (e.g. camMoveAt) share this single implementation. MUST stay identical to the Rust
 *  `ease` - export is the source of truth, this only drives the live preview. An unparseable name
 *  falls through to smooth, matching `valid_easing`'s coercion. */
export function ease(name: string, p: number): number {
  const c = Math.min(1, Math.max(0, p));
  if (name === "linear") return c;
  const spr = springOf(name);
  if (spr) return spring(spr[0], spr[1], spr[2], c);
  if (name === "ease_in") return c * c;
  if (name === "ease_out") return c * (2 - c);
  if (name === "ease_in_out") return c < 0.5 ? 2 * c * c : 1 - 2 * (1 - c) * (1 - c);
  const cub = parseCubic(name);
  if (cub) return evalCubic(cub[0], cub[1], cub[2], cub[3], c);
  return c * c * (3 - 2 * c); // smooth (default)
}

/** TS mirror of Rust `fit_durations` (export/camera/mod.rs), which `LayoutTrack` applies to every
 *  segment at construction. Shrinks a segment's entry+exit proportionally so they fit inside its
 *  own span. Without it a segment shorter than its own entry never reaches its own scene, while
 *  `rawSegAt` hands that unreached scene to whatever blends off it next - so the frame JUMPED at
 *  the boundary (~950px on a 3840-wide frame for a 200ms segment with a 350ms entry) - and an
 *  entry+exit that together outlast the span ran the exit underneath the entry.
 *  `Math.fround` mirrors the f32 arithmetic the export truncates, so both sides pick the same ms. */
export function fitDurations(tin: number, tout: number, span: number): [number, number] {
  const total = tin + tout;
  if (total <= span || total === 0) return [tin, tout];
  const f = Math.fround(span / total);
  return [Math.trunc(Math.fround(tin * f)), Math.trunc(Math.fround(tout * f))];
}

/** One segment's transitions AS THE TRACK SEES THEM - fitted into its span, never the raw doc
 *  values. Every reader below goes through this, exactly as Rust's `Seg` stores only the fitted
 *  pair, so the entry branch, the exit branch and the successor's "is its entry still running"
 *  test can never disagree about how long a transition is. */
const fittedOf = (s: LayoutSeg): [number, number] =>
  fitDurations(s.transition_ms, s.transition_out_ms, Math.max(0, s.end_ms - s.start_ms));

const KNOWN: LayoutPresetName[] = ["screen", "camera", "presenter", "screen_only", "camera_only"];
const presetOf = (presets: LayoutPresets, layout: string): LayoutPresetDto =>
  presets[(KNOWN as string[]).includes(layout) ? (layout as LayoutPresetName) : "screen"];

const lerpN = (a: number, b: number, t: number) => a + (b - a) * t;
/** Mirrors the Rust `lp` (export/scene/mod.rs): continuous fields (rect/radius/alpha/ring width)
 *  cross-fade, but `ring_color` snaps to `b`'s color once `t` crosses the midpoint - an RGB lerp
 *  mid-transition looks muddy, and the export never blends it either. */
const lerpRect = (a: PanelRectDto, b: PanelRectDto, t: number): PanelRectDto => ({
  rect: [lerpN(a.rect[0], b.rect[0], t), lerpN(a.rect[1], b.rect[1], t), lerpN(a.rect[2], b.rect[2], t), lerpN(a.rect[3], b.rect[3], t)],
  radius: lerpN(a.radius, b.radius, t), alpha: lerpN(a.alpha, b.alpha, t),
  ring_px: lerpN(a.ring_px, b.ring_px, t), ring_color: t < 0.5 ? a.ring_color : b.ring_color,
});

/** Collapse a resolved (screen, cam) PanelRect pair to the PreviewLayout the canvas draws.
 *  `canvas` is passed through unchanged from the caller's own `PreviewLayout` (this cross-fade
 *  never changes the backing-store size, only the panel rects within it). */
function toPreviewLayout(screen: PanelRectDto, cam: PanelRectDto, canvas: [number, number]): PreviewLayout {
  return {
    screen: screen.rect, radius: screen.radius, screenAlpha: screen.alpha,
    cam: cam.alpha > 0.004
      ? [cam.rect[0], cam.rect[1], cam.rect[2], cam.rect[3], cam.radius,
         cam.ring_px, cam.ring_color[0], cam.ring_color[1], cam.ring_color[2]]
      : null,
    camAlpha: cam.alpha, canvas,
  };
}

/** The index of the segment active at `t` (latest-starting one containing it), or -1 in a gap. */
function activeIdx(ordered: LayoutSeg[], t: number): number {
  let idx = -1;
  for (let k = 0; k < ordered.length; k++) { const s = ordered[k]; if (t >= s.start_ms && t < s.end_ms) idx = k; }
  return idx;
}

/** The raw segment active at `t` with NO cross-fade (the last one CONTAINING `t`), or `null` in a
 *  gap - used to compute a fade's "from"/"to" endpoint before resolving it to panels. */
const rawSegAt = (ordered: LayoutSeg[], t: number): LayoutSeg | null => {
  const idx = activeIdx(ordered, t);
  return idx < 0 ? null : ordered[idx];
};

/** `seg`'s resolved panels: its own per-segment override from `presets.segs` (by id) when one
 *  exists, else its preset's panels - `null` (a gap, or a segment before backend `segs` catches
 *  up to an edit) always means the base `screen` preset with no override to look up. Falls back
 *  PER PANEL, not just per segment, so a `segs` entry that only overrides one field (not produced
 *  today, but not assumed against either) still shows something sane. Zero pose math: this only
 *  picks which already-resolved rect Rust computed. */
export function resolvedPanelsFor(seg: LayoutSeg | null, presets: LayoutPresets): ResolvedPanels {
  const preset = presetOf(presets, seg?.layout ?? "screen");
  const entry = seg ? presets.segs.find((e) => e.id === seg.id) : undefined;
  return { screen: entry?.screen ?? preset.screen, cam: entry?.cam ?? preset.cam };
}

/** The active layout at output time `t`, mirroring LayoutTrack::scene_at exactly: a segment is
 *  active only INSIDE `[start, end)`; outside every segment (a gap, or before/after all) falls
 *  back to the base `screen` preset ("empty means default"). Latest-starting containing segment
 *  wins. On entering a segment, cross-fades from whatever was active just before it over that
 *  segment's own transition_ms/easing - FITTED into its span first (`fittedOf`), as Rust does at
 *  construction; over the last transition_out_ms before its end it fades
 *  toward whatever follows it, landing on that successor exactly AT end_ms - unless the successor's
 *  OWN entry blend covers end_ms, in which case that entry wins and the exit stands down (one blend
 *  at a time). Returns null when presets haven't loaded (caller falls back). */
export function layoutAt(segs: LayoutSeg[], presets: LayoutPresets | null, t: number, canvas: [number, number]): PreviewLayout | null {
  if (!presets) return null;
  const ordered = [...segs].sort((a, b) => a.start_ms - b.start_ms);
  const idx = activeIdx(ordered, t);
  if (idx < 0) { const b = resolvedPanelsFor(null, presets); return toPreviewLayout(b.screen, b.cam, canvas); } // gap/outside -> screen

  const s = ordered[idx];
  const cur = resolvedPanelsFor(s, presets);
  const blend = (from: ResolvedPanels, to: ResolvedPanels, f: number) =>
    toPreviewLayout(lerpRect(from.screen, to.screen, f), lerpRect(from.cam, to.cam, f), canvas);

  const [tin, tout] = fittedOf(s);
  const elapsed = t - s.start_ms;
  if (tin > 0 && elapsed < tin) {
    const from = resolvedPanelsFor(rawSegAt(ordered, Math.max(0, s.start_ms - 1)), presets); // active just before this seg
    return blend(from, cur, ease(s.easing, elapsed / tin));
  }
  const exitFrom = s.end_ms - tout;
  if (tout > 0 && t >= exitFrom) {
    const next = activeIdx(ordered, s.end_ms);
    const nextTin = next >= 0 ? fittedOf(ordered[next])[0] : 0;
    const nextEntryWins = next >= 0 && nextTin > 0 && s.end_ms - ordered[next].start_ms < nextTin;
    if (!nextEntryWins) {
      const to = resolvedPanelsFor(next >= 0 ? ordered[next] : null, presets);
      return blend(cur, to, ease(s.easing_out, (t - exitFrom) / tout));
    }
  }
  return toPreviewLayout(cur.screen, cur.cam, canvas);
}
