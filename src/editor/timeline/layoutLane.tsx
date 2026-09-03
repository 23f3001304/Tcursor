import { useMemo } from "react";
import type { LayoutSeg } from "../../lib/edit";
import type { LayoutPresets } from "../../lib/ipc";
import { layoutRegions, transitionRampPct } from "./layers";
import { resolvedPanelsFor, type ResolvedPanels } from "./layoutTrack";
import { LayoutThumb } from "./LayoutThumb";

const prettyLayout = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

export type LayoutRegion = LayoutSeg & { layer: number; panels: ResolvedPanels | null };

/** The Layout lane's own regions for `Timeline.tsx`/`RegionRows`: `doc.layout` minus the empty
 *  "screen" default (the one layout hidden as a pill - deleting a pill, or leaving a gap, is how
 *  you get back to it), laid into non-overlapping rows, each carrying its RESOLVED panels
 *  (`resolvedPanelsFor`, T34 L2's per-segment channel) so `layoutLabel` below can draw a thumbnail
 *  without re-deriving any pose math of its own - `panels` is `null` while `presets` hasn't loaded,
 *  the same gate `LayoutInspector` uses. Split out of `Timeline.tsx` (T34 L4) so that file didn't
 *  have to grow past its line cap to add the thumbnail. */
export function useLayoutLaneRegions(layout: LayoutSeg[], presets: LayoutPresets | null): LayoutRegion[] {
  const nonScreen = useMemo(() => layout.filter((s) => s.layout !== "screen"), [layout]);
  return useMemo(
    () => layoutRegions(nonScreen).map((l) => ({ ...l, panels: presets ? resolvedPanelsFor(l, presets) : null })),
    [nonScreen, presets]
  );
}

/** `RegionRows.renderLabel` for the Layout lane (T34 L4): the thumbnail replaces the plain
 *  `IconAspectRatio` icon this used to show unconditionally; the text reads "Custom" for a segment
 *  carrying its own `arrangement`, else the preset name it's still bound to. Stays a stable
 *  module-scope function (not a per-render closure) - `panels` rides on the region object itself
 *  (`useLayoutLaneRegions`), which is exactly why that hook exists, so `RegionRows`' `React.memo`
 *  isn't defeated by a fresh `renderLabel` identity every render (same rule `zoomLabel`/`fxLabel`
 *  follow in `Timeline.tsx`). */
export const layoutLabel = (l: LayoutRegion) => (
  <>
    <LayoutThumb panels={l.panels} />
    {l.arrangement ? "Custom" : prettyLayout(l.layout)}
  </>
);

export const layoutExtraStyle = (l: { transition_ms: number; transition_out_ms: number }, s: number, e: number) =>
  ({ "--fin": `${transitionRampPct(l.transition_ms, e - s)}%`, "--fout": `${transitionRampPct(l.transition_out_ms, e - s)}%` } as React.CSSProperties);
