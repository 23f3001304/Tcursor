import type { LayoutSeg } from "../../../shared/edit";
import type {
  LayoutPresets,
  LayoutPresetName,
  LayoutPresetDto,
  PanelRectDto,
  PreviewLayout,
} from "../../../shared/ipc";
import { evalCubic, parseCubic } from "../../../shared/math/cubicBezier";
import { spring, springOf } from "../../../shared/math/spring";
import { evalKeys, parseKeys } from "../../motion/keys";
import { fitPanel, lerpN, spanAt, type SpanState } from "../../stage/canvas/sourceSpans";

export interface ResolvedPanels {
  screen: PanelRectDto;
  cam: PanelRectDto;
}

export function ease(name: string, p: number): number {
  const c = Math.min(1, Math.max(0, p));
  if (name === "linear") return c;
  const spr = springOf(name);
  if (spr) return spring(spr[0], spr[1], spr[2], c);
  const keys = parseKeys(name);
  if (keys) return evalKeys(keys, c);
  if (name === "ease_in") return c * c;
  if (name === "ease_out") return c * (2 - c);
  if (name === "ease_in_out") return c < 0.5 ? 2 * c * c : 1 - 2 * (1 - c) * (1 - c);
  const cub = parseCubic(name);
  if (cub) return evalCubic(cub[0], cub[1], cub[2], cub[3], c);
  return c * c * (3 - 2 * c);
}

export function fitDurations(tin: number, tout: number, span: number): [number, number] {
  const total = tin + tout;
  if (total <= span || total === 0) return [tin, tout];
  const f = Math.fround(span / total);
  return [Math.trunc(Math.fround(tin * f)), Math.trunc(Math.fround(tout * f))];
}

const fittedOf = (s: LayoutSeg): [number, number] =>
  fitDurations(s.transition_ms, s.transition_out_ms, Math.max(0, s.end_ms - s.start_ms));

const KNOWN: LayoutPresetName[] = ["screen", "camera", "presenter", "screen_only", "camera_only"];
const presetOf = (presets: LayoutPresets, layout: string): LayoutPresetDto =>
  presets[(KNOWN as string[]).includes(layout) ? (layout as LayoutPresetName) : "screen"];

const lerpRect = (a: PanelRectDto, b: PanelRectDto, t: number): PanelRectDto => ({
  rect: [
    lerpN(a.rect[0], b.rect[0], t),
    lerpN(a.rect[1], b.rect[1], t),
    lerpN(a.rect[2], b.rect[2], t),
    lerpN(a.rect[3], b.rect[3], t),
  ],
  radius: lerpN(a.radius, b.radius, t),
  alpha: lerpN(a.alpha, b.alpha, t),
  ring_px: lerpN(a.ring_px, b.ring_px, t),
  ring_color: t < 0.5 ? a.ring_color : b.ring_color,
});

function toPreviewLayout(
  screen: PanelRectDto,
  cam: PanelRectDto,
  canvas: [number, number],
  span: SpanState,
): PreviewLayout {
  return {
    screen: fitPanel(screen.rect, span.fit),
    radius: screen.radius,
    screenAlpha: screen.alpha,
    src: span.src,
    cam:
      cam.alpha > 0.004
        ? [
            cam.rect[0],
            cam.rect[1],
            cam.rect[2],
            cam.rect[3],
            cam.radius,
            cam.ring_px,
            cam.ring_color[0],
            cam.ring_color[1],
            cam.ring_color[2],
          ]
        : null,
    camAlpha: cam.alpha,
    canvas,
  };
}

function activeIdx(ordered: LayoutSeg[], t: number): number {
  let idx = -1;
  for (let k = 0; k < ordered.length; k++) {
    const s = ordered[k];
    if (t >= s.start_ms && t < s.end_ms) idx = k;
  }
  return idx;
}

const rawSegAt = (ordered: LayoutSeg[], t: number): LayoutSeg | null => {
  const idx = activeIdx(ordered, t);
  return idx < 0 ? null : ordered[idx];
};

export function resolvedPanelsFor(seg: LayoutSeg | null, presets: LayoutPresets): ResolvedPanels {
  const preset = presetOf(presets, seg?.layout ?? "screen");
  const entry = seg ? presets.segs.find((e) => e.id === seg.id) : undefined;
  return { screen: entry?.screen ?? preset.screen, cam: entry?.cam ?? preset.cam };
}

export function layoutAt(
  segs: LayoutSeg[],
  presets: LayoutPresets | null,
  t: number,
  canvas: [number, number],
): PreviewLayout | null {
  if (!presets) return null;
  const span = spanAt(presets.spans ?? [], t, ease);
  const ordered = [...segs].sort((a, b) => a.start_ms - b.start_ms);
  const idx = activeIdx(ordered, t);
  if (idx < 0) {
    const b = resolvedPanelsFor(null, presets);
    return toPreviewLayout(b.screen, b.cam, canvas, span);
  }

  const s = ordered[idx];
  const cur = resolvedPanelsFor(s, presets);
  const blend = (from: ResolvedPanels, to: ResolvedPanels, f: number) =>
    toPreviewLayout(lerpRect(from.screen, to.screen, f), lerpRect(from.cam, to.cam, f), canvas, span);

  const [tin, tout] = fittedOf(s);
  const elapsed = t - s.start_ms;
  if (tin > 0 && elapsed < tin) {
    const from = resolvedPanelsFor(rawSegAt(ordered, Math.max(0, s.start_ms - 1)), presets);
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
  return toPreviewLayout(cur.screen, cur.cam, canvas, span);
}
