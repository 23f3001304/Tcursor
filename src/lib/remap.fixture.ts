import { buildTimeMap, type TimeMap } from "./remap";

/** The Rust parity fixture (`export/remap_tests.rs::fixture`): trim 500..9000 of a 10000 ms clip,
 *  cuts [1000,2000) and [4000,4500), 2x on [2500,3500), 0.5x on [6000,8000). Every expected value in
 *  `remap.test.ts`, `remapDoc.test.ts` and `playbackRemap.test.ts` is computed from this exact map on
 *  the Rust side; a TS result that disagrees is wrong, not the table. */
export function fixtureMap(): TimeMap {
  return buildTimeMap({ in_ms: 500, out_ms: 9000 },
    [{ id: "c0", start_ms: 1000, end_ms: 2000 }, { id: "c1", start_ms: 4000, end_ms: 4500 }],
    [{ id: "s0", start_ms: 2500, end_ms: 3500, factor: 2 }, { id: "s1", start_ms: 6000, end_ms: 8000, factor: 0.5 }],
    10_000);
}
