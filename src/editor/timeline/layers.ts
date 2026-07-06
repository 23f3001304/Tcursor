/** Greedy interval-partitioning: sort regions by start, then place each on the lowest layer
 *  whose previous region ends at or before this one's start. Overlapping regions therefore
 *  land on new layers - so the timeline can stack overlapping effects on separate rows. */
export function layoutRegions<T extends { start_ms: number; end_ms: number }>(regions: T[]): (T & { layer: number })[] {
  const ends: number[] = []; // the last end time placed on each layer
  return [...regions].sort((a, b) => a.start_ms - b.start_ms).map((r) => {
    let layer = ends.findIndex((e) => e <= r.start_ms);
    if (layer < 0) { layer = ends.length; ends.push(r.end_ms); } else { ends[layer] = r.end_ms; }
    return { ...r, layer };
  });
}
