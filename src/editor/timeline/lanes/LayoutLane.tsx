import { useMemo } from "react";
import { IconAspectRatio } from "@tabler/icons-react";
import type { LayoutSeg } from "../../../shared/edit";
import type { LayoutPresets } from "../../../shared/ipc";
import { arrThumb, THUMB_W, THUMB_H } from "../model/arrThumb";
import { layoutRegions, transitionRampPct } from "../model/layers";
import { resolvedPanelsFor, type ResolvedPanels } from "../model/layoutTrack";

const prettyLayout = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

export function LayoutThumb({
  panels,
  w = THUMB_W,
  h = THUMB_H,
  className = "e-laythumb",
}: {
  panels: ResolvedPanels | null;
  w?: number;
  h?: number;
  className?: string;
}) {
  const t = arrThumb(panels);
  if (!t.screen && !t.cam) return <IconAspectRatio size={12} className={className} />;
  return (
    <svg className={className} viewBox={`0 0 ${THUMB_W} ${THUMB_H}`} width={w} height={h} aria-hidden="true">
      {t.screen && (
        <rect
          className="e-laythumb-screen"
          x={t.screen.x}
          y={t.screen.y}
          width={t.screen.w}
          height={t.screen.h}
          rx={0.8}
        />
      )}
      {t.cam && (
        <rect className="e-laythumb-cam" x={t.cam.x} y={t.cam.y} width={t.cam.w} height={t.cam.h} rx={0.8} />
      )}
    </svg>
  );
}

export type LayoutRegion = LayoutSeg & { layer: number; panels: ResolvedPanels | null };

export function useLayoutLaneRegions(layout: LayoutSeg[], presets: LayoutPresets | null): LayoutRegion[] {
  const nonScreen = useMemo(() => layout.filter((s) => s.layout !== "screen"), [layout]);
  return useMemo(
    () =>
      layoutRegions(nonScreen).map((l) => ({ ...l, panels: presets ? resolvedPanelsFor(l, presets) : null })),
    [nonScreen, presets],
  );
}

export const layoutLabel = (l: LayoutRegion) => (
  <>
    <LayoutThumb panels={l.panels} />
    {l.arrangement ? "Custom" : prettyLayout(l.layout)}
  </>
);

export const layoutExtraStyle = (
  l: { transition_ms: number; transition_out_ms: number },
  s: number,
  e: number,
) =>
  ({
    "--fin": `${transitionRampPct(l.transition_ms, e - s)}%`,
    "--fout": `${transitionRampPct(l.transition_out_ms, e - s)}%`,
  }) as React.CSSProperties;
