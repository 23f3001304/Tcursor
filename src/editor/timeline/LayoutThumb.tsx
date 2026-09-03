import { IconAspectRatio } from "@tabler/icons-react";
import { arrThumb, THUMB_W, THUMB_H } from "./arrThumb";
import type { ResolvedPanels } from "./layoutTrack";

/** The `arrThumb` schematic drawn as a tiny inline SVG - a `--e-layout`-outlined screen rect plus a
 *  filled `--e-cam` dot for the webcam, literally the segment's OWN resolved panel fractions
 *  (see `arrThumb.ts`) scaled into one small box. Shared by the Layout lane's pill (24x14, its own
 *  icon slot - `Timeline.tsx` via `layoutLane.tsx`) and `LayoutInspector`'s header (48x28) - same
 *  `viewBox`, just a bigger rendered `width`/`height`, so the two can never draw a different
 *  schematic for the same segment.
 *
 *  `panels === null` (presets not loaded yet, or no segment) falls back to the plain aspect-ratio
 *  icon the pill showed before this task, rather than leaving an empty gap in the label. */
export function LayoutThumb({ panels, w = THUMB_W, h = THUMB_H, className = "e-laythumb" }: {
  panels: ResolvedPanels | null; w?: number; h?: number; className?: string;
}) {
  const t = arrThumb(panels);
  if (!t.screen && !t.cam) return <IconAspectRatio size={12} className={className} />;
  return (
    <svg className={className} viewBox={`0 0 ${THUMB_W} ${THUMB_H}`} width={w} height={h} aria-hidden="true">
      {t.screen && <rect className="e-laythumb-screen" x={t.screen.x} y={t.screen.y} width={t.screen.w} height={t.screen.h} rx={0.8} />}
      {t.cam && <rect className="e-laythumb-cam" x={t.cam.x} y={t.cam.y} width={t.cam.w} height={t.cam.h} rx={0.8} />}
    </svg>
  );
}
