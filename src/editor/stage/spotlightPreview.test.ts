import { describe, expect, it } from "vitest";
import { newSpotlightSimState, resolveSpotlight, type SpotlightInput } from "./spotlightPreview";
import type { EffectRegion } from "../../lib/edit";

const region = (over: Partial<EffectRegion> = {}): EffectRegion => ({
  id: "e0", kind: "spotlight", start_ms: 3000, end_ms: 6000, fade_in_ms: 250, fade_out_ms: 250, layer: 0, ...over,
});

const input = (effects: EffectRegion[]): SpotlightInput => ({
  effects, on: false, params: { dim: 0.6, radius: 0.13, feather: 0.1, mode: "classic", tint: [130, 90, 255] },
});

describe("resolveSpotlight - backward-scrub elapsed clamp (mirrors Rust saturating_sub)", () => {
  it("reproduces the M7 scenario: exit past end_ms arms a transition, scrubbing back before it doesn't blow alpha out of range", () => {
    const effects = [region()]; // [3000, 6000], 250ms fades
    const sim = newSpotlightSimState();
    // Warm up: land inside the region, fully faded in (natural alpha 1).
    const warm = resolveSpotlight(input(effects), 3300, sim);
    expect(warm!.alpha).toBe(1);

    // Scrub past end_ms: driver goes e0 -> null, arming an outgoing (fade_out) transition right
    // at the moment of exit; transitionFrom snapshots the alpha at that instant (1).
    const exited = resolveSpotlight(input(effects), 6010, sim);
    expect(exited).not.toBeNull();
    expect(sim.transitionStart).toBe(6010);
    const transitionFrom = sim.transitionFrom!;
    expect(transitionFrom).toBe(1);

    // Scrub BACKWARDS to 3050 - just after the region's own start, where ITS OWN fade-in alpha
    // ("natural") is only ~0.2, far from transitionFrom (1). Re-entering the region from "no
    // driver" does not re-arm (matches Rust's identical `driver.is_some()` guard), so this stale
    // transition (armed at 6010) is still in effect. Un-clamped, elapsed = 3050-6010 = -2960,
    // e = -11.84, e*e*(3-2e) = ~3740 - multiplied against (natural - transitionFrom) = -0.8, that
    // drives sim.alpha to roughly -2991, which the settings-union `max(alpha, on?1:0)` then
    // floors to a hard 0 (the "sign flip -> zero" case from the evidence): the spotlight would
    // wrongly vanish entirely instead of holding at a sane value.
    const resolved = resolveSpotlight(input(effects), 3050, sim);
    expect(resolved).not.toBeNull();
    expect(resolved!.alpha).toBeGreaterThanOrEqual(0);
    expect(resolved!.alpha).toBeLessThanOrEqual(1.001);
    // Clamped elapsed (0, since 3050 is before transitionStart) pins alpha at transitionFrom -
    // the correct, Rust-mirrored "the transition hasn't started yet" reading - not a blowup, and
    // not a wrong hard zero.
    expect(resolved!.alpha).toBeCloseTo(transitionFrom, 5);
  });

  it("clamps elapsed to exactly 0 (not negative) the instant the playhead sits before transitionStart", () => {
    // Directly exercise the clamp: hand-construct a sim mid-transition and resolve at a time
    // before its start, isolated from the driver-handoff bookkeeping in the test above.
    const effects = [region({ start_ms: 0, end_ms: 10_000, fade_in_ms: 1 })];
    const sim = newSpotlightSimState();
    sim.driver = "e0";
    sim.alpha = 0.4;
    sim.transitionFrom = 0.4;
    sim.transitionStart = 5000;
    sim.transitionDur = 250;

    const resolved = resolveSpotlight(input(effects), 1000, sim); // 4000ms BEFORE transitionStart
    // elapsed clamps to 0 -> smoothstep(0) = 0 -> alpha holds at transitionFrom exactly.
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

    const resolved = resolveSpotlight(input(effects), 5125, sim); // halfway through the transition
    expect(resolved!.alpha).toBeGreaterThan(0.4);
    expect(resolved!.alpha).toBeLessThan(1);
  });
});
