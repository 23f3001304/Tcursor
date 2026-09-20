import type { CSSProperties } from "react";
import type { Clip } from "../../../shared/edit";
import { clipOutMs, type TimeMap } from "../../../shared/math/remap";
import { layoutRegions, transitionRampPct } from "./layers";

export interface ClipRegion {
  id: string;
  start_ms: number;
  end_ms: number;
  layer: number;
  order: number;
  outMs: number;
  transition_in_ms: number;
}

export function clipRegions(clips: Clip[], map: TimeMap): ClipRegion[] {
  return layoutRegions(
    clips.map((c, i) => ({
      id: c.id,
      start_ms: c.src_in_ms,
      end_ms: c.src_out_ms,
      order: i + 1,
      outMs: clipOutMs(map, i),
      transition_in_ms: c.transition_in_ms,
    })),
  );
}

export const clipLabel = (c: ClipRegion) => `${c.order} - ${(c.outMs / 1000).toFixed(1)}s`;

export const clipExtraStyle = (c: ClipRegion, s: number, e: number): CSSProperties =>
  ({ "--fin": `${transitionRampPct(c.transition_in_ms, e - s)}%` }) as CSSProperties;

export function dropIndex(clips: Clip[], draggedId: string, atMs: number): number | null {
  const i = clips.findIndex((c) => c.id !== draggedId && atMs >= c.src_in_ms && atMs < c.src_out_ms);
  return i < 0 || !clips.some((c) => c.id === draggedId) ? null : i;
}
