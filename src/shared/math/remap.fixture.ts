import { buildTimeMap, type TimeMap } from "./remap";

export const fixtureParts = () =>
  ({
    trim: { in_ms: 500, out_ms: 9000 },
    cuts: [
      { id: "c0", start_ms: 1000, end_ms: 2000 },
      { id: "c1", start_ms: 4000, end_ms: 4500 },
    ],
    speed: [
      { id: "s0", start_ms: 2500, end_ms: 3500, factor: 2 },
      { id: "s1", start_ms: 6000, end_ms: 8000, factor: 0.5 },
    ],
  }) as const;

export function fixtureMap(): TimeMap {
  const p = fixtureParts();
  return buildTimeMap(p.trim, [...p.cuts], [...p.speed], [], 10_000);
}

export function fractionalFixtureMap(): TimeMap {
  return buildTimeMap(
    { in_ms: 0, out_ms: 0 },
    [{ id: "fc0", start_ms: 3000, end_ms: 4000 }],
    [{ id: "fs0", start_ms: 0, end_ms: 2000, factor: 1.5 }],
    [],
    10_000,
  );
}

export function clipsFixtureMap(): TimeMap {
  const p = fixtureParts();
  return buildTimeMap(
    p.trim,
    [...p.cuts],
    [...p.speed],
    [
      { id: "cl1", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: 0 },
      { id: "cl0", src_in_ms: 500, src_out_ms: 4000, transition_in_ms: 0 },
    ],
    10_000,
  );
}
