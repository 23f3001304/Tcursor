import { describe, expect, it } from "vitest";
import {
  ALPHA_LINEAR_SPOT_MODES,
  newSpotlightSimState,
  resolveSpotlight,
  spotAlphaPlan,
  spotlightEffectsKey,
  type ResolvedSpotlight,
  type SpotlightInput,
} from "./spotlightPreview";
import type { EffectRegion } from "../../../shared/edit";

const region = (over: Partial<EffectRegion> = {}): EffectRegion => ({
  id: "e0",
  kind: "spotlight",
  start_ms: 3000,
  end_ms: 6000,
  fade_in_ms: 250,
  fade_out_ms: 250,
  layer: 0,
  ...over,
});

const input = (effects: EffectRegion[]): SpotlightInput => ({
  effects,
  on: false,
  params: { dim: 0.6, radius: 0.13, feather: 0.1, mode: "classic", tint: [130, 90, 255] },
});

describe("resolveSpotlight - backward-scrub elapsed clamp (mirrors Rust saturating_sub)", () => {
  it("reproduces the M7 scenario: exit past end_ms arms a transition, scrubbing back before it doesn't blow alpha out of range", () => {
    const effects = [region()];
    const sim = newSpotlightSimState();
    const warm = resolveSpotlight(input(effects), 3300, sim);
    expect(warm!.alpha).toBe(1);

    const exited = resolveSpotlight(input(effects), 6010, sim);
    expect(exited).not.toBeNull();
    expect(sim.transitionStart).toBe(6010);
    const transitionFrom = sim.transitionFrom!;
    expect(transitionFrom).toBe(1);

    const resolved = resolveSpotlight(input(effects), 3050, sim);
    expect(resolved).not.toBeNull();
    expect(resolved!.alpha).toBeGreaterThanOrEqual(0);
    expect(resolved!.alpha).toBeLessThanOrEqual(1.001);
    expect(resolved!.alpha).toBeCloseTo(transitionFrom, 5);
  });

  it("clamps elapsed to exactly 0 (not negative) the instant the playhead sits before transitionStart", () => {
    const effects = [region({ start_ms: 0, end_ms: 10_000, fade_in_ms: 1 })];
    const sim = newSpotlightSimState();
    sim.driver = "e0";
    sim.alpha = 0.4;
    sim.transitionFrom = 0.4;
    sim.transitionStart = 5000;
    sim.transitionDur = 250;

    const resolved = resolveSpotlight(input(effects), 1000, sim);
    expect(resolved!.alpha).toBeCloseTo(0.4, 5);
  });

  it("still eases normally (unclamped) once time moves forward past transitionStart", () => {
    const effects = [region({ start_ms: 0, end_ms: 10_000, fade_in_ms: 1 })];
    const sim = newSpotlightSimState();
    sim.driver = "e0";
    sim.alpha = 0.4;
    sim.transitionFrom = 0.4;
    sim.transitionStart = 5000;
    sim.transitionDur = 250;

    const resolved = resolveSpotlight(input(effects), 5125, sim);
    expect(resolved!.alpha).toBeGreaterThan(0.4);
    expect(resolved!.alpha).toBeLessThan(1);
  });
});

describe("spotlightEffectsKey", () => {
  it("is stable for content-identical arrays even with a different array/object reference", () => {
    const a = [region({ id: "e1" }), region({ id: "e2", start_ms: 9000, end_ms: 12000 })];
    const b = [region({ id: "e1" }), region({ id: "e2", start_ms: 9000, end_ms: 12000 })];
    expect(a).not.toBe(b);
    expect(spotlightEffectsKey(a)).toBe(spotlightEffectsKey(b));
  });

  it("is empty-but-deterministic for an empty list", () => {
    expect(spotlightEffectsKey([])).toBe(spotlightEffectsKey([]));
  });

  it("changes when a region is added or removed", () => {
    const before = [region({ id: "e1" })];
    const after = [...before, region({ id: "e2", start_ms: 9000, end_ms: 12000 })];
    expect(spotlightEffectsKey(before)).not.toBe(spotlightEffectsKey(after));
  });

  it("changes when any spotlight-relevant field changes (id/start/end/fade/mode/dim/radius/feather)", () => {
    const base = region({ mode: "classic", dim: 0.6, radius: 0.13, feather: 0.1 });
    const baseKey = spotlightEffectsKey([base]);
    expect(spotlightEffectsKey([{ ...base, id: "e9" }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, start_ms: 3001 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, end_ms: 6001 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, fade_in_ms: 251 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, fade_out_ms: 251 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, mode: "halo" }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, dim: 0.61 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, radius: 0.14 }])).not.toBe(baseKey);
    expect(spotlightEffectsKey([{ ...base, feather: 0.11 }])).not.toBe(baseKey);
  });

  it("is unaffected by a field it doesn't track (layer)", () => {
    const base = region({ layer: 0 });
    expect(spotlightEffectsKey([{ ...base, layer: 5 }])).toBe(spotlightEffectsKey([base]));
  });

  it("is sensitive to array order (a same-set reorder still counts as a change)", () => {
    const a = [region({ id: "e1" }), region({ id: "e2", start_ms: 9000, end_ms: 12000 })];
    const b = [a[1], a[0]];
    expect(spotlightEffectsKey(a)).not.toBe(spotlightEffectsKey(b));
  });
});

describe("resolveSpotlight - sim reset clears a mid-transition freeze (M9)", () => {
  it("driving a sim into an outgoing transition then resetting it clears the spotlight immediately at an outside-span ms", () => {
    const effects = [region()];
    const sim = newSpotlightSimState();
    resolveSpotlight(input(effects), 3300, sim);
    const stillOn = resolveSpotlight(input([]), 3300, sim);
    expect(stillOn).not.toBeNull();
    expect(sim.transitionStart).toBe(3300);

    const resolved = resolveSpotlight(input([]), 3300, newSpotlightSimState());
    expect(resolved).toBeNull();
  });
});

const resolvedSpot = (over: Partial<ResolvedSpotlight> = {}): ResolvedSpotlight => ({
  alpha: 0.4,
  mode: "classic",
  dim: 0.6,
  radius: 0.13,
  feather: 0.1,
  tint: [130, 90, 255],
  ...over,
});

describe("spotAlphaPlan", () => {
  it("splits a linear-mode, spotlight-only overlay into a reference request + a live blit alpha", () => {
    const p = spotAlphaPlan(resolvedSpot({ alpha: 0.4 }), false);
    expect(p).toEqual({ requestAlpha: 1, drawAlpha: 0.4, separable: true });
  });

  it("keeps the backend round-trip when the overlay also carries click rings", () => {
    const p = spotAlphaPlan(resolvedSpot({ alpha: 0.4 }), true);
    expect(p).toEqual({ requestAlpha: 0.4, drawAlpha: 1, separable: false });
  });

  it("keeps the backend round-trip for a mode whose alpha response is not proportional", () => {
    for (const mode of ["halo", "nebula", "blur"]) {
      expect(spotAlphaPlan(resolvedSpot({ mode, alpha: 0.4 }), false)).toEqual({
        requestAlpha: 0.4,
        drawAlpha: 1,
        separable: false,
      });
    }
  });

  it("covers every linear mode", () => {
    for (const mode of ALPHA_LINEAR_SPOT_MODES) {
      expect(spotAlphaPlan(resolvedSpot({ mode }), false).separable).toBe(true);
    }
    expect([...ALPHA_LINEAR_SPOT_MODES].sort()).toEqual(["breathing", "classic", "vignette"]);
  });

  it("draws a separable overlay at 0 once nothing is lit, without waiting for a round-trip", () => {
    expect(spotAlphaPlan(null, false)).toEqual({ requestAlpha: 0, drawAlpha: 0, separable: false });
  });

  it("clamps the blit alpha into 0..1", () => {
    expect(spotAlphaPlan(resolvedSpot({ alpha: 1.6 }), false).drawAlpha).toBe(1);
    expect(spotAlphaPlan(resolvedSpot({ alpha: -0.2 }), false).drawAlpha).toBe(0);
  });

  it("is a full ramp across a fade, not the two or three steps a bucketed round-trip gave", () => {
    const alphas = [0, 0.2, 0.4, 0.6, 0.8, 1].map(
      (a) => spotAlphaPlan(resolvedSpot({ alpha: a }), false).drawAlpha,
    );
    expect(alphas).toEqual([0, 0.2, 0.4, 0.6, 0.8, 1]);
    const requests = new Set(
      [0, 0.2, 0.4, 0.6, 0.8, 1].map((a) => spotAlphaPlan(resolvedSpot({ alpha: a }), false).requestAlpha),
    );
    expect(requests).toEqual(new Set([1]));
  });
});

describe("resolveSpotlight ignores masks", () => {
  it("a blur region on a higher layer never drives the spotlight", () => {
    const sim = newSpotlightSimState();
    const effects = [region({ id: "m0", kind: "blur", start_ms: 0, end_ms: 10_000, layer: 9 }), region()];
    expect(resolveSpotlight(input(effects), 1000, sim)).toBeNull();
    expect(resolveSpotlight(input(effects), 4500, sim)).not.toBeNull();
  });
  it("the effects key changes when a region's kind changes", () => {
    expect(spotlightEffectsKey([region()])).not.toBe(spotlightEffectsKey([region({ kind: "blur" })]));
  });
});
