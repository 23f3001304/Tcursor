import { describe, it, expect } from "vitest";
import {
  RIPPLE_LIFE_MS, stylesMirrored, overlayNeedsClicks, activeRippleHits, rippleAlpha,
  rippleRadiusPx, rippleThicknessPx, shockwaveRadiusPx, shockwaveThicknessPx, SHOCKWAVE_ALPHA_MUL,
  drawRipplePreview, drawMirroredRipples,
} from "./ripplePreview";
import type { ClickSample } from "../../lib/ipc";

/** A ctx stub that throws if any drawing method is touched - proves the no-op branches below
 *  genuinely draw nothing rather than merely returning early after some partial work. */
const untouchableCtx = new Proxy({}, {
  get(_t, prop) { throw new Error(`drawRipplePreview touched ctx.${String(prop)} on a no-op path`); },
}) as unknown as CanvasRenderingContext2D;

const clk = (t: number, x: number, y: number): ClickSample => ({ t, x, y });

describe("activeRippleHits (mirrors export/fx/clickfx.rs::hits_at)", () => {
  it("includes a click exactly at its start (Rust: et >= e.t, inclusive)", () => {
    const hits = activeRippleHits([clk(1000, 0.5, 0.5)], 1000);
    expect(hits).toEqual([{ x: 0.5, y: 0.5, progress: 0 }]);
  });

  it("excludes a click exactly at its life boundary (Rust: et - e.t < life_ms, EXCLUSIVE end)", () => {
    const hits = activeRippleHits([clk(0, 0.1, 0.1)], RIPPLE_LIFE_MS);
    expect(hits).toEqual([]);
  });

  it("computes progress as elapsed/life for a click mid-life", () => {
    const hits = activeRippleHits([clk(0, 0.2, 0.3)], RIPPLE_LIFE_MS / 2);
    expect(hits[0].progress).toBeCloseTo(0.5, 10);
  });

  it("excludes a click that hasn't happened yet (future t)", () => {
    expect(activeRippleHits([clk(500, 0, 0)], 100)).toEqual([]);
  });

  it("keeps every simultaneously-alive click, in order", () => {
    const hits = activeRippleHits([clk(0, 0, 0), clk(100, 1, 1)], 300);
    expect(hits.map(h => h.x)).toEqual([0, 1]);
  });
});

describe("rippleAlpha (mirrors fx.wgsl line 169: clamp(1-h.z,0,1) * clamp(intensity,0,1))", () => {
  it("is full intensity at progress 0", () => {
    expect(rippleAlpha(0, 1)).toBe(1);
    expect(rippleAlpha(0, 0.8)).toBeCloseTo(0.8, 10);
  });

  it("is exactly 0.25 at progress 0.5, intensity 0.5 (clickfx.rs test: fade_alpha(0.5,0.5)==0.25)", () => {
    expect(rippleAlpha(0.5, 0.5)).toBeCloseTo(0.25, 10);
  });

  it("is 0 at progress 1 (fully faded)", () => {
    expect(rippleAlpha(1, 1)).toBe(0);
  });

  it("clamps progress and intensity outside their normal ranges", () => {
    expect(rippleAlpha(1.5, 1)).toBe(0); // past life -> clamped to progress 1
    expect(rippleAlpha(-0.5, 1)).toBe(1); // before life -> clamped to progress 0
    expect(rippleAlpha(0, 2)).toBe(1); // intensity clamped to 1
  });
});

describe("rippleRadiusPx (mirrors fx.wgsl line 172: clamp(h.z,0,1) * oh * 0.06)", () => {
  it("is 0 at progress 0", () => {
    expect(rippleRadiusPx(0, 1000)).toBe(0);
  });

  it("is exactly oh*0.06 at progress 1 (oh=1000 -> 60px)", () => {
    expect(rippleRadiusPx(1, 1000)).toBeCloseTo(60, 10);
  });

  it("is exactly half that at progress 0.5 (oh=1000 -> 30px)", () => {
    expect(rippleRadiusPx(0.5, 1000)).toBeCloseTo(30, 10);
  });

  it("clamps progress past 1 (never overshoots the max radius)", () => {
    expect(rippleRadiusPx(2, 1000)).toBeCloseTo(60, 10);
  });
});

describe("rippleThicknessPx (mirrors fx.wgsl line 173: max(oh*0.006, 1.0))", () => {
  it("is oh*0.006 for a normal output size (oh=1000 -> 6px)", () => {
    expect(rippleThicknessPx(1000)).toBeCloseTo(6, 10);
  });

  it("floors at 1px for a small output (oh=100 -> 0.6px would round to 1px)", () => {
    expect(rippleThicknessPx(100)).toBe(1);
  });

  it("is exactly the floor boundary at oh ~166.67 (oh*0.006 == 1.0)", () => {
    expect(rippleThicknessPx(1000 / 6)).toBeCloseTo(1, 10);
  });
});

describe("shockwaveRadiusPx (mirrors fx.wgsl line 186: clamp(h.z,0,1) * oh * 0.09)", () => {
  it("is 0 at progress 0", () => {
    expect(shockwaveRadiusPx(0, 1000)).toBe(0);
  });

  it("is exactly oh*0.09 at progress 1 (oh=1000 -> 90px)", () => {
    expect(shockwaveRadiusPx(1, 1000)).toBeCloseTo(90, 10);
  });

  it("is exactly half that at progress 0.5 (oh=1000 -> 45px)", () => {
    expect(shockwaveRadiusPx(0.5, 1000)).toBeCloseTo(45, 10);
  });
});

describe("shockwaveThicknessPx (mirrors fx.wgsl line 187: max(oh*0.008, 1.0))", () => {
  it("is oh*0.008 for a normal output size (oh=1000 -> 8px)", () => {
    expect(shockwaveThicknessPx(1000)).toBeCloseTo(8, 10);
  });

  it("floors at 1px for a small output (oh=50 -> 0.4px would round to 1px)", () => {
    expect(shockwaveThicknessPx(50)).toBe(1);
  });
});

describe("SHOCKWAVE_ALPHA_MUL (mirrors fx.wgsl line 188: ... * a * 0.5)", () => {
  it("is exactly 0.5", () => {
    expect(SHOCKWAVE_ALPHA_MUL).toBe(0.5);
  });
});

describe("stylesMirrored", () => {
  it("covers exactly the default style (ripple) plus shockwave - the two prioritized this pass", () => {
    expect(stylesMirrored.has("ripple")).toBe(true);
    expect(stylesMirrored.has("shockwave")).toBe(true);
  });

  it("does not cover the four styles that fall back to the backend overlay", () => {
    for (const s of ["pulse", "glow", "particles", "neon", "none"]) {
      expect(stylesMirrored.has(s)).toBe(false);
    }
  });
});

describe("overlayNeedsClicks (fxOverlay.ts's fallback gate)", () => {
  it("is false for a mirrored style - ripplePreview.ts draws its clicks instead", () => {
    expect(overlayNeedsClicks("ripple")).toBe(false);
    expect(overlayNeedsClicks("shockwave")).toBe(false);
  });

  it("is true for every unmirrored, real click-fx style - falls back to the overlay", () => {
    for (const s of ["pulse", "glow", "particles", "neon"]) {
      expect(overlayNeedsClicks(s)).toBe(true);
    }
  });

  it("is false for \"none\" - nothing to draw either way", () => {
    expect(overlayNeedsClicks("none")).toBe(false);
  });
});

describe("drawRipplePreview no-op paths (never touch ctx)", () => {
  it("does nothing for a style outside stylesMirrored", () => {
    expect(() => drawRipplePreview(untouchableCtx, [{ x: 1, y: 1, progress: 0.5 }], "glow", [255, 255, 255], 1, 1000))
      .not.toThrow();
  });

  it("does nothing for an empty hit list", () => {
    expect(() => drawRipplePreview(untouchableCtx, [], "ripple", [255, 255, 255], 1, 1000)).not.toThrow();
  });

  it("does nothing for a non-positive output height", () => {
    expect(() => drawRipplePreview(untouchableCtx, [{ x: 1, y: 1, progress: 0.5 }], "ripple", [255, 255, 255], 1, 0))
      .not.toThrow();
  });
});

describe("drawMirroredRipples no-op paths (never touch ctx)", () => {
  const identity = (x: number, y: number): [number, number] => [x, y];
  const activeClick = [clk(0, 0.5, 0.5)]; // alive at now=0 (dt=0 < RIPPLE_LIFE_MS)

  // Regression (coordinator-flagged Critical): with defaults (enabled:true, style:Ripple),
  // toggling "Click animations" off still drew ripple rings in the live preview - the export
  // correctly suppresses them (fx_state.rs's `render` gate), and fxOverlay.ts's own
  // requestFxOverlay already treats `enabled` as the FIRST check for exactly this reason.
  // `enabled` must be checked here too, not just at some call site that might forget it.
  it("does nothing when clickfx is disabled, even for an active click on a mirrored style", () => {
    expect(() => drawMirroredRipples(untouchableCtx, activeClick, 0, false, "ripple", [255, 255, 255], 1, identity, 100, 100, 200, 200))
      .not.toThrow();
  });

  it("does nothing for a style outside stylesMirrored, even when enabled", () => {
    expect(() => drawMirroredRipples(untouchableCtx, activeClick, 0, true, "glow", [255, 255, 255], 1, identity, 100, 100, 200, 200))
      .not.toThrow();
  });

  it("does nothing for an empty click list, even when enabled", () => {
    expect(() => drawMirroredRipples(untouchableCtx, [], 0, true, "ripple", [255, 255, 255], 1, identity, 100, 100, 200, 200))
      .not.toThrow();
  });
});
