import { useRegionDrag, type BeginDrag, type Drag } from "../hooks/input/useRegionDrag";

export function useLaneDrag<T extends { id: string; start_ms: number; end_ms: number; layer: number }>(
  regions: T[],
  dur: number,
  trackRef: React.RefObject<HTMLDivElement | null>,
  rowHeightPx: number,
  onCommit: (id: string, start: number, end: number, layer: number) => void,
  onSel: (id: string) => void,
): { drag: Drag | null; beginDrag: BeginDrag; rows: number } {
  const { drag, beginDrag } = useRegionDrag(regions, dur, trackRef, rowHeightPx, onCommit, onSel);
  const maxLayer = Math.max(0, ...regions.map((r) => r.layer), drag ? drag.layer : 0);
  return { drag, beginDrag, rows: regions.length ? maxLayer + 1 : 0 };
}
