import { describe, it, expect } from "vitest";
import {
  RIPPLE_LIFE_MS, RIPPLE_RINGS, stylesMirrored, overlayNeedsClicks, activeRippleHits, easeOut,
  rippleAlpha, rippleRadiusPx, rippleThicknessPx, shockwaveRadiusPx, shockwaveThicknessPx,
  pulseRadiusPx, SHOCKWAVE_ALPHA_MUL, smoothstep, drawRipplePreview, drawMirroredRipples,
} from "./ripplePreview";
import type { ClickSample } from "../../lib/ipc";

/** Throws if ANY ctx member is touched - proves the no-op branches below really draw nothing. */
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

describe("easeOut (mirrors fx_clicks.wgsl::fx_ease / clickfx.rs::ease_out)", () => {
  // The SAME five points the Rust test `ease_out_is_pinned_at_five_points` pins.
  it.each([[0, 0], [0.25, 0.578125], [0.5, 0.875], [0.75, 0.984375], [1, 1]])(
    "easeOut(%f) === %f", (p, want) => { expect(easeOut(p)).toBeCloseTo(want, 10); });

  it("clamps outside the life", () => {
    expect(easeOut(-1)).toBe(0);
    expect(easeOut(2)).toBe(1);
  });
});

describe("rippleAlpha (mirrors fx_clicks.wgsl::fx_alpha / clickfx.rs::fade_alpha)", () => {
  // The SAME five points the Rust test `fade_alpha_is_pinned_at_five_points` pins: full through
  // the 0.55 hold, then the smoothstep release, exactly 0 at the end (no pop-off).
  it.each([[0, 1], [0.25, 1], [0.5, 1], [0.75, 0.5829904], [1, 0]])(
    "rippleAlpha(%f, 1) === %f", (p, want) => { expect(rippleAlpha(p, 1)).toBeCloseTo(want, 6); });

  it("scales by intensity inside the hold (Rust: fade_alpha(0.5, 0.5) == 0.5)", () => {
    expect(rippleAlpha(0.5, 0.5)).toBeCloseTo(0.5, 10);
  });

  it("clamps progress and intensity outside their normal ranges", () => {
    expect(rippleAlpha(1.5, 1)).toBe(0);
    expect(rippleAlpha(-0.5, 1)).toBe(1);
    expect(rippleAlpha(0, 2)).toBe(1);
  });
});

describe("smoothstep (the GPU builtin)", () => {
  it("is 0 below e0, 1 above e1 and 0.5 at the midpoint", () => {
    expect(smoothstep(0, 1, -1)).toBe(0);
    expect(smoothstep(0, 1, 2)).toBe(1);
    expect(smoothstep(0, 1, 0.5)).toBeCloseTo(0.5, 10);
  });
});

describe("rippleRadiusPx (mirrors fx_clicks.wgsl: fx_ease(p) * oh * 0.06)", () => {
  it("runs 0 -> oh*0.06 over the life (oh=1000 -> 0 then 60px)", () => {
    expect(rippleRadiusPx(0, 1000)).toBe(0);
    expect(rippleRadiusPx(1, 1000)).toBeCloseTo(60, 10);
  });

  it("is EASED, not linear, at progress 0.5 (oh=1000 -> 52.5px, not 30)", () => {
    expect(rippleRadiusPx(0.5, 1000)).toBeCloseTo(52.5, 10);
  });

  it("clamps progress past 1 (never overshoots the max radius)", () => {
    expect(rippleRadiusPx(2, 1000)).toBeCloseTo(60, 10);
  });
});

describe("RIPPLE_RINGS (fx_clicks.wgsl's three trailing rings)", () => {
  it("is three rings launched 0 / 90 / 180 ms apart, each thinner and fainter", () => {
    expect(RIPPLE_RINGS.map(r => r.offset)).toEqual([0, 0.15, 0.30]);
    expect(RIPPLE_RINGS.map(r => r.thick)).toEqual([0.006, 0.0045, 0.003]);
    expect(RIPPLE_RINGS.map(r => r.gain)).toEqual([1, 0.7, 0.45]);
  });

  it("offsets are those launch delays over the 600ms life", () => {
    expect(RIPPLE_RINGS[1].offset * RIPPLE_LIFE_MS).toBeCloseTo(90, 10);
    expect(RIPPLE_RINGS[2].offset * RIPPLE_LIFE_MS).toBeCloseTo(180, 10);
  });
});

describe("shockwaveRadiusPx (mirrors fx_clicks.wgsl: fx_ease(p) * oh * 0.09)", () => {
  it("runs 0 -> oh*0.09 over the life (oh=1000 -> 0 then 90px)", () => {
    expect(shockwaveRadiusPx(0, 1000)).toBe(0);
    expect(shockwaveRadiusPx(1, 1000)).toBeCloseTo(90, 10);
  });

  it("is eased at progress 0.5 (oh=1000 -> 78.75px, not the linear 45)", () => {
    expect(shockwaveRadiusPx(0.5, 1000)).toBeCloseTo(78.75, 10);
  });
});

describe("ring thicknesses and the shockwave gain", () => {
  it("are oh*0.006 and oh*0.008 at a normal output size (oh=1000 -> 6px, 8px)", () => {
    expect(rippleThicknessPx(1000)).toBeCloseTo(6, 10);
    expect(shockwaveThicknessPx(1000)).toBeCloseTo(8, 10);
  });

  it("floor at 1px on a tiny preview canvas, like the shader's max(.., 1.0)", () => {
    expect(rippleThicknessPx(100)).toBe(1);
    expect(shockwaveThicknessPx(50)).toBe(1);
  });

  it("keep the shockwave ring additive at exactly 0.5x", () => {
    expect(SHOCKWAVE_ALPHA_MUL).toBe(0.5);
  });
});

describe("pulseRadiusPx (mirrors fx_clicks.wgsl: oh * (0.01 + 0.025 * fx_ease(p)))", () => {
  it("starts at 1% of height and ends at 3.5% (oh=1000 -> 10px then 35px)", () => {
    expect(pulseRadiusPx(0, 1000)).toBeCloseTo(10, 10);
    expect(pulseRadiusPx(1, 1000)).toBeCloseTo(35, 10);
  });

  it("is eased between them (oh=1000, p=0.5 -> 10 + 25*0.875 = 31.875px)", () => {
    expect(pulseRadiusPx(0.5, 1000)).toBeCloseTo(31.875, 10);
  });
});

describe("stylesMirrored and overlayNeedsClicks (fxOverlay.ts's fallback gate)", () => {
  it("mirrors ripple (the default), shockwave and pulse, which then need no overlay hits", () => {
    for (const s of ["ripple", "shockwave", "pulse"]) {
      expect(stylesMirrored.has(s)).toBe(true);
      expect(overlayNeedsClicks(s)).toBe(false);
    }
  });

  it("leaves the other three to the backend overlay, which must still carry their hits", () => {
    for (const s of ["glow", "particles", "neon"]) {
      expect(stylesMirrored.has(s)).toBe(false);
      expect(overlayNeedsClicks(s)).toBe(true);
    }
  });

  it("treats \"none\" as neither: nothing to draw either way", () => {
    expect(stylesMirrored.has("none")).toBe(false);
    expect(overlayNeedsClicks("none")).toBe(false);
  });
});

describe("drawRipplePreview no-op paths (never touch ctx)", () => {
  it("does nothing for a style outside stylesMirrored", () => {
    expect(() => drawRipplePreview(untouchableCtx, [{ x: 1, y: 1, progress: 0.5 }], "glow", [255, 255, 255], 1, 1000))
      .not.toThrow();
  });

  it("does nothing for an empty hit list or a non-positive output height", () => {
    expect(() => drawRipplePreview(untouchableCtx, [], "ripple", [255, 255, 255], 1, 1000)).not.toThrow();
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
