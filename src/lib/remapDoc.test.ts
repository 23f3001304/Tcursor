import { describe, expect, it } from "vitest";
import type { EditDoc, EffectRegion } from "./edit";
import { identityMap } from "./remap";
import { fixtureMap } from "./remap.fixture";
import { remapDoc } from "./remapDoc";

const base = (): EditDoc => ({
  version: 2, trim: { in_ms: 0, out_ms: 0 }, cuts: [], zooms: [], speed: [], layout: [], effects: [], camera_moves: [],
  aspect: "16:9" as EditDoc["aspect"], settings: {} as EditDoc["settings"],
});
const spot = (id: string, start_ms: number, end_ms: number): EffectRegion =>
  ({ id, kind: "spotlight", start_ms, end_ms, fade_in_ms: 250, fade_out_ms: 250, layer: 0 });

describe("remapDoc (parity with edit::remap_doc)", () => {
  it("moves regions to the output clock and keeps their durations and ids", () => {
    const d = base();
    d.zooms.push({ id: "z0", start_ms: 2200, end_ms: 3200, target: "cursor", scale: 2, easing: "smooth", zoom_in_ms: 350, zoom_out_ms: 450, layer: 0 });
    const r = remapDoc(d, fixtureMap());
    expect([r.zooms[0].start_ms, r.zooms[0].end_ms]).toEqual([700, 1350]);
    expect([r.zooms[0].zoom_in_ms, r.zooms[0].zoom_out_ms]).toEqual([350, 450]);
    expect(r.zooms[0].id).toBe("z0");
  });

  it("drops a region entirely inside a cut and shrinks one straddling it", () => {
    const d = base();
    d.effects.push(spot("e0", 1100, 1900), spot("e1", 800, 2200));
    const r = remapDoc(d, fixtureMap());
    expect(r.effects.map((e) => [e.id, e.start_ms, e.end_ms])).toEqual([["e1", 300, 700]]);
  });

  it("maps camera moves and clears the consumed fields", () => {
    const d = base();
    d.trim = { in_ms: 500, out_ms: 9000 };
    d.cuts.push({ id: "c0", start_ms: 1000, end_ms: 2000 });
    d.speed.push({ id: "s0", start_ms: 2500, end_ms: 3500, factor: 2 });
    d.camera_moves.push({ id: "m0", t_ms: 1500, x: 0.5, y: 0.5, size: 0.3, easing: "smooth", shape: "layout", roundness: 0.12 });
    const r = remapDoc(d, fixtureMap());
    expect(r.camera_moves[0].t_ms).toBe(500);
    expect(r.trim).toEqual({ in_ms: 0, out_ms: 0 });
    expect(r.cuts).toEqual([]);
    expect(r.speed).toEqual([]);
  });

  it("is the identity on regions for a plain map", () => {
    const d = base();
    d.layout.push({ id: "l0", start_ms: 100, end_ms: 900, layout: "camera", transition_ms: 350, easing: "smooth", transition_out_ms: 0, easing_out: "smooth" });
    expect(remapDoc(d, identityMap(10_000)).layout).toEqual(d.layout);
  });
});
