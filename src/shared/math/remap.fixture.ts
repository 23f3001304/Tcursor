import { buildTimeMap, type TimeMap } from "./remap";

export function fixtureMap(): TimeMap {
  return buildTimeMap(
    { in_ms: 500, out_ms: 9000 },
    [
      { id: "c0", start_ms: 1000, end_ms: 2000 },
      { id: "c1", start_ms: 4000, end_ms: 4500 },
    ],
    [
      { id: "s0", start_ms: 2500, end_ms: 3500, factor: 2 },
      { id: "s1", start_ms: 6000, end_ms: 8000, factor: 0.5 },
    ],
    10_000,
  );
}
