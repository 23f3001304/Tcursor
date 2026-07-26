export function layoutRegions<T extends { start_ms: number; end_ms: number; layer?: number }>(
  regions: T[]
): (T & { layer: number })[] {
  const sorted = [...regions].sort((a, b) => a.start_ms - b.start_ms);
  const layerOccupancy: Map<number, { start_ms: number; end_ms: number }[]> = new Map();
  const assigned: (T & { layer: number })[] = [];

  // Phase 1: Keep explicit layers (if present)
  for (const r of sorted) {
    if (typeof r.layer === "number" && r.layer >= 0) {
      const l = r.layer;
      if (!layerOccupancy.has(l)) layerOccupancy.set(l, []);
      layerOccupancy.get(l)!.push({ start_ms: r.start_ms, end_ms: r.end_ms });
      assigned.push({ ...r, layer: l });
    }
  }

  // Phase 2: Assign unassigned regions to the lowest non-overlapping layer
  for (const r of sorted) {
    if (typeof r.layer !== "number" || r.layer < 0) {
      let l = 0;
      while (true) {
        const intervals = layerOccupancy.get(l) || [];
        const overlaps = intervals.some((inv) => r.start_ms < inv.end_ms && r.end_ms > inv.start_ms);
        if (!overlaps) {
          intervals.push({ start_ms: r.start_ms, end_ms: r.end_ms });
          layerOccupancy.set(l, intervals);
          assigned.push({ ...r, layer: l });
          break;
        }
        l++;
      }
    }
  }

  return assigned.sort((a, b) => a.start_ms - b.start_ms);
}

