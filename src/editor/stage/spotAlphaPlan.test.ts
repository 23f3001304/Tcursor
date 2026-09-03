import { describe, expect, it } from "vitest";
import { ALPHA_LINEAR_SPOT_MODES, spotAlphaPlan } from "./spotlightPreview";
import type { ResolvedSpotlight } from "./spotlightPreview";

// In its own file rather than appended to spotlightPreview.test.ts, which is already near the
// size limit. Covers only the alpha SPLIT between the backend request and the client-side blit -
// the sim/handoff behaviour of `resolveSpotlight` itself stays in the sibling file.

const resolved = (over: Partial<ResolvedSpotlight> = {}): ResolvedSpotlight => ({
  alpha: 0.4, mode: "classic", dim: 0.6, radius: 0.13, feather: 0.1, tint: [130, 90, 255], ...over,
});

describe("spotAlphaPlan", () => {
  it("splits a linear-mode, spotlight-only overlay into a reference request + a live blit alpha", () => {
    const p = spotAlphaPlan(resolved({ alpha: 0.4 }), false);
    expect(p).toEqual({ requestAlpha: 1, drawAlpha: 0.4, separable: true });
  });

  it("keeps the backend round-trip when the overlay also carries click rings", () => {
    // Pulse/Glow/Neon/Particles rings are baked into this same PNG - fading the layer for the
    // spotlight would fade them too, so this case must stay bit-identical to the old behaviour.
    const p = spotAlphaPlan(resolved({ alpha: 0.4 }), true);
    expect(p).toEqual({ requestAlpha: 0.4, drawAlpha: 1, separable: false });
  });

  it("keeps the backend round-trip for a mode whose alpha response is not proportional", () => {
    // Halo adds an intensity-scaled ring that ignores alpha; Nebula's shader never reads it at
    // all; Blur mixes toward a blurred sample by `t`. Scaling those client-side would look
    // different from the export, so they are excluded on purpose.
    for (const mode of ["halo", "nebula", "blur"]) {
      expect(spotAlphaPlan(resolved({ mode, alpha: 0.4 }), false))
        .toEqual({ requestAlpha: 0.4, drawAlpha: 1, separable: false });
    }
  });

  it("covers every linear mode", () => {
    for (const mode of ALPHA_LINEAR_SPOT_MODES) {
      expect(spotAlphaPlan(resolved({ mode }), false).separable).toBe(true);
    }
    expect([...ALPHA_LINEAR_SPOT_MODES].sort()).toEqual(["breathing", "classic", "vignette"]);
  });

  it("draws a separable overlay at 0 once nothing is lit, without waiting for a round-trip", () => {
    expect(spotAlphaPlan(null, false)).toEqual({ requestAlpha: 0, drawAlpha: 0, separable: false });
  });

  it("clamps the blit alpha into 0..1", () => {
    expect(spotAlphaPlan(resolved({ alpha: 1.6 }), false).drawAlpha).toBe(1);
    expect(spotAlphaPlan(resolved({ alpha: -0.2 }), false).drawAlpha).toBe(0);
  });

  it("is a full ramp across a fade, not the two or three steps a bucketed round-trip gave", () => {
    // The bug: a 250ms fade at FX_BUCKET_MS = 40ms got ~6 backend updates at BEST, and fewer
    // whenever a single-flighted request outlived the fade - so it read as a pop, not a fade.
    const alphas = [0, 0.2, 0.4, 0.6, 0.8, 1].map((a) => spotAlphaPlan(resolved({ alpha: a }), false).drawAlpha);
    expect(alphas).toEqual([0, 0.2, 0.4, 0.6, 0.8, 1]);
    // ...and every one of those frames reuses the SAME cached image (requestAlpha never varies).
    const requests = new Set([0, 0.2, 0.4, 0.6, 0.8, 1].map((a) => spotAlphaPlan(resolved({ alpha: a }), false).requestAlpha));
    expect(requests).toEqual(new Set([1]));
  });
});
