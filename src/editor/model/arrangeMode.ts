import type { LayoutSeg } from "../../shared/edit";

export interface ArrangeMode {
  on: boolean;
  segId: string | null;
}

export type ArrangeEvent =
  { kind: "select"; segId: string | null } | { kind: "arrange" } | { kind: "escape" } | { kind: "gone" };

export const NO_ARRANGE: ArrangeMode = { on: false, segId: null };

export function nextArrangeMode(prev: ArrangeMode, ev: ArrangeEvent): ArrangeMode {
  switch (ev.kind) {
    case "select":
      if (!ev.segId) return NO_ARRANGE;
      return prev.on && prev.segId === ev.segId ? prev : { on: true, segId: ev.segId };
    case "arrange":
      return prev.segId ? { on: true, segId: prev.segId } : prev;
    case "escape":
      return prev.on ? { on: false, segId: prev.segId } : prev;
    case "gone":
      return NO_ARRANGE;
  }
}

export function arrangeSeekMs(seg: LayoutSeg, timeMs: number): number | null {
  if (timeMs >= seg.start_ms && timeMs < seg.end_ms) return null;
  return Math.min(seg.start_ms + seg.transition_ms, Math.max(seg.start_ms, seg.end_ms - 1));
}
