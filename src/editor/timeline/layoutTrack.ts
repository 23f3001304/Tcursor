import type { LayoutSeg } from "../../lib/edit";
import type { LayoutPresets, LayoutPresetName, LayoutPresetDto, PanelRectDto, PreviewLayout } from "../../lib/ipc";

/** Easing mirror of the export's curves (crate::export::camera::ease): linear / smoothstep /
 *  ease-out-back spring / quadratic ease-in / ease-out / ease-in-out. Exported so other per-frame
 *  TS mirrors (e.g. camMoveAt) share this single implementation. MUST stay identical to the Rust
 *  `ease` - export is the source of truth, this only drives the live preview. */
export function ease(name: string, p: number): number {
  const c = Math.min(1, Math.max(0, p));
  if (name === "linear") return c;
  if (name === "spring") { const k = 1.70158, q = c - 1; return 1 + (k + 1) * q * q * q + k * q * q; }
  if (name === "ease_in") return c * c;
  if (name === "ease_out") return c * (2 - c);
  if (name === "ease_in_out") return c < 0.5 ? 2 * c * c : 1 - 2 * (1 - c) * (1 - c);
  return c * c * (3 - 2 * c); // smooth (default)
}

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

/** The preset active at `t` with NO cross-fade: the last segment CONTAINING `t` ([start, end)),
 *  else the base `screen`. Used to compute a fade's "from"../. */
function rawPresetAt(ordered: LayoutSeg[], presets: LayoutPresets, t: number): LayoutPresetDto {
  let found: LayoutPresetDto | null = null;
  for (const s of ordered) { if (t >= s.start_ms && t < s.end_ms) found = presetOf(presets, s.layout); }
  return found ?? presets.screen;
}

/** The active layout at output time `t`, mirroring LayoutTrack::scene_at exactly: a segment is
 *  active only INSIDE `[start, end)`; outside every segment (a gap, or before/after all) falls
 *  back to the base `screen` preset ("empty means default"). Latest-starting containing segment
 *  wins. On entering a segment, cross-fades from whatever was active just before it over that
 *  segment's own transition_ms/easing. Returns null when presets haven't loaded (caller falls back). */
export function layoutAt(segs: LayoutSeg[], presets: LayoutPresets | null, t: number, canvas: [number, number]): PreviewLayout | null {
  if (!presets) return null;
  const ordered = [...segs].sort((a, b) => a.start_ms - b.start_ms);
  let idx = -1;
  for (let k = 0; k < ordered.length; k++) { const s = ordered[k]; if (t >= s.start_ms && t < s.end_ms) idx = k; }
  if (idx < 0) { const b = presets.screen; return toPreviewLayout(b.screen, b.cam, canvas); } // gap/outside -> screen

  const s = ordered[idx];
  const cur = presetOf(presets, s.layout);
  const elapsed = t - s.start_ms;
  if (s.transition_ms <= 0 || elapsed >= s.transition_ms) return toPreviewLayout(cur.screen, cur.cam, canvas);

  const from = rawPresetAt(ordered, presets, Math.max(0, s.start_ms - 1)); // active just before this seg
  const f = ease(s.easing, elapsed / s.transition_ms);
  return toPreviewLayout(lerpRect(from.screen, cur.screen, f), lerpRect(from.cam, cur.cam, f), canvas);
}
