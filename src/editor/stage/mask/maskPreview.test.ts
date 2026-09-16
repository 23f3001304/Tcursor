import { describe, it, expect } from "vitest";
import type { EffectRegion } from "../../../shared/edit";
import type { PreviewLayout } from "../../../shared/ipc";
import { blurSigmaFor, maskDraws, type MaskPx } from "./maskPreview";

const SCREEN: PreviewLayout = {
  screen: [0, 0, 1, 1],
  radius: 0,
  cam: null,
  canvas: [1920, 1080],
  screenAlpha: 1,
  camAlpha: 0,
};

const PRESENTER: PreviewLayout = {
  screen: [0.05, 0.1, 0.7, 0.7],
  radius: 0,
  cam: [0.77, 0.574, 0.1875, 0.3333, 0, 0, 0, 0, 0],
  canvas: [1920, 1080],
  screenAlpha: 1,
  camAlpha: 1,
};

const mask = (rect: [number, number, number, number]): EffectRegion => ({
  id: "m0",
  kind: "blur",
  start_ms: 0,
  end_ms: 10000,
  fade_in_ms: 250,
  fade_out_ms: 250,
  layer: 0,
  rect,
});

const RUST_POSE: Record<string, [number, number, number]> = {
  identity: [960, 540, 1],
  centred: [960, 540, 2.2],
  corner: [120, 90, 2.2],
};

function poseIn(l: PreviewLayout, pose: string) {
  const [cxPx, cyPx, scale] = RUST_POSE[pose];
  const [w, h] = l.canvas;
  const frac = (px: number, lo: number, span: number) => Math.min(1, Math.max(0, (px - lo) / span));
  return {
    cx: frac(cxPx, l.screen[0] * w, l.screen[2] * w),
    cy: frac(cyPx, l.screen[1] * h, l.screen[3] * h),
    scale,
  };
}

const POSES = {
  identity: poseIn(SCREEN, "identity"),
  centred: poseIn(SCREEN, "centred"),
  corner: poseIn(SCREEN, "corner"),
};

describe("maskDraws mirrors masks_at", () => {
  // GENERATED from the Rust side: see the note below. Do not hand-edit a number.
  const rows: [string, string, [number, number, number, number], number[]][] = [
    ["screen", "identity", [0.0, 0.0, 0.2, 0.2], [0.0, 0.0, 384.0, 216.0, 12.96, 10.8]],
    ["screen", "identity", [0.35, 0.4, 0.3, 0.2], [672.0, 432.0, 1248.0, 648.0, 12.96, 10.8]],
    ["screen", "identity", [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 1920.0, 1080.0, 64.8, 10.8]],
    ["screen", "identity", [0.49, 0.49, 0.01, 0.01], [941.0, 529.0, 960.0, 540.0, 0.66, 10.8]],
    ["screen", "centred", [0.0, 0.0, 0.2, 0.2], [-1151.3402, -647.78, -306.8041, -172.668, 28.5067, 10.8]],
    ["screen", "centred", [0.35, 0.4, 0.3, 0.2], [326.5979, 302.444, 1593.4021, 777.556, 28.5067, 10.8]],
    ["screen", "centred", [0.0, 0.0, 1.0, 1.0], [-1151.3402, -647.78, 3071.3401, 1727.78, 142.5336, 10.8]],
    ["screen", "centred", [0.49, 0.49, 0.01, 0.01], [918.2131, 515.8045, 960.0, 540.0, 1.4517, 10.8]],
    ["screen", "corner", [0.0, 0.0, 0.2, 0.2], [0.0, 0.0, 844.5361, 475.112, 28.5067, 10.8]],
    ["screen", "corner", [0.35, 0.4, 0.3, 0.2], [1477.9381, 950.2241, 2744.7422, 1425.3361, 28.5067, 10.8]],
    ["screen", "corner", [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 4222.6802, 2375.5601, 142.5336, 10.8]],
    ["screen", "corner", [0.49, 0.49, 0.01, 0.01], [2069.5532, 1163.5845, 2111.3401, 1187.78, 1.4517, 10.8]],
    ["presenter", "identity", [0.0, 0.0, 0.2, 0.2], [96.0, 108.0, 365.0, 259.0, 9.06, 10.8]],
    ["presenter", "identity", [0.35, 0.4, 0.3, 0.2], [566.0, 410.0, 970.0, 562.0, 9.12, 10.8]],
    ["presenter", "identity", [0.0, 0.0, 1.0, 1.0], [96.0, 108.0, 1440.0, 864.0, 45.36, 10.8]],
    ["presenter", "identity", [0.49, 0.49, 0.01, 0.01], [755.0, 478.0, 768.0, 486.0, 0.48, 10.8]],
    ["presenter", "centred", [0.0, 0.0, 0.2, 0.2], [-940.2062, -410.224, -348.5911, -78.0855, 19.9283, 10.8]],
    ["presenter", "centred", [0.35, 0.4, 0.3, 0.2], [93.4708, 254.0529, 981.9931, 588.3911, 20.0603, 10.8]],
    ["presenter", "centred", [0.0, 0.0, 1.0, 1.0], [-940.2062, -410.224, 2015.67, 1252.668, 99.7735, 10.8]],
    ["presenter", "centred", [0.49, 0.49, 0.01, 0.01], [509.1409, 403.6252, 537.7319, 421.222, 1.0558, 10.8]],
    ["presenter", "corner", [0.0, 0.0, 0.2, 0.2], [211.134, 237.556, 802.7491, 569.6945, 19.9283, 10.8]],
    ["presenter", "corner", [0.35, 0.4, 0.3, 0.2], [1244.811, 901.833, 2133.3333, 1236.171, 20.0603, 10.8]],
    ["presenter", "corner", [0.0, 0.0, 1.0, 1.0], [211.134, 237.556, 3167.0103, 1900.4481, 99.7735, 10.8]],
    [
      "presenter",
      "corner",
      [0.49, 0.49, 0.01, 0.01],
      [1660.4811, 1051.4053, 1689.0721, 1069.0021, 1.0558, 10.8],
    ],
  ];

  it("pins the same twenty four projections the Rust table pins", () => {
    for (const [lay, pose, rect, want] of rows) {
      const layout = lay === "screen" ? SCREEN : PRESENTER;
      const got = maskDraws([mask(rect)], layout, poseIn(layout, pose), 1920, 1080, 5000, 0.6);
      expect(got, `${lay}/${pose}/${rect}`).toHaveLength(1);
      const m = got[0];
      const have = [m.mn[0], m.mn[1], m.mx[0], m.mx[1], m.r, m.featherPx];
      for (let k = 0; k < 6; k++) {
        expect(
          Math.abs(have[k] - want[k]),
          `${lay}/${pose}/${rect} field ${k}: want ${want[k]} got ${have[k]}`,
        ).toBeLessThanOrEqual(k === 5 ? 1e-3 : 1);
      }
    }
  });

  it("drops a mask when the screen panel is hidden, and a spotlight always", () => {
    const hidden = { ...SCREEN, screenAlpha: 0.2 };
    expect(maskDraws([mask([0.1, 0.1, 0.2, 0.2])], hidden, POSES.identity, 1920, 1080, 5000, 0.6)).toEqual(
      [],
    );
    const spot: EffectRegion = { ...mask([0.1, 0.1, 0.2, 0.2]), kind: "spotlight" };
    expect(maskDraws([spot], SCREEN, POSES.identity, 1920, 1080, 5000, 0.6)).toEqual([]);
  });

  it("fades each region on its own, in layer order", () => {
    const a = { ...mask([0.1, 0.1, 0.2, 0.2]), end_ms: 2000, fade_in_ms: 1000 };
    const b: EffectRegion = {
      ...mask([0.5, 0.5, 0.2, 0.2]),
      id: "m1",
      kind: "pixelate",
      end_ms: 2000,
      fade_in_ms: 200,
      layer: 1,
    };
    const got = maskDraws([b, a], SCREEN, POSES.identity, 1920, 1080, 400, 0.6);
    expect(got.map((m: MaskPx) => m.kind)).toEqual([1, 2]);
    expect(got[0].alpha).toBeCloseTo(0.4, 3);
    expect(got[1].alpha).toBe(1);
  });

  it("takes the kind's default strength, the global dim and the short-side roundness", () => {
    const [blur] = maskDraws([mask([0.1, 0.1, 0.2, 0.2])], SCREEN, POSES.identity, 1920, 1080, 5000, 0.62);
    expect(blur.amountPx).toBeCloseTo(1080 * 0.02, 3);
    expect(blur.featherPx).toBeCloseTo(1080 * 0.01, 3);
    expect(blur.r).toBeCloseTo(216 * 0.06, 3);
    const pix: EffectRegion = { ...mask([0.1, 0.1, 0.2, 0.2]), kind: "pixelate" };
    expect(maskDraws([pix], SCREEN, POSES.identity, 1920, 1080, 5000, 0.62)[0].amountPx).toBeCloseTo(
      1080 * 0.018,
      3,
    );
    const hi: EffectRegion = { ...mask([0.1, 0.1, 0.2, 0.2]), kind: "highlight" };
    expect(maskDraws([hi], SCREEN, POSES.identity, 1920, 1080, 5000, 0.62)[0].dim).toBeCloseTo(0.62, 6);
  });
});

describe("blurSigmaFor", () => {
  it("is the same three box approximation the export uses", () => {
    expect(blurSigmaFor(1)).toBeCloseTo(Math.SQRT2, 5);
    expect(blurSigmaFor(24)).toBeCloseTo(Math.sqrt((49 * 49 - 1) / 4), 3);
    expect(blurSigmaFor(0)).toBe(blurSigmaFor(1));
  });
});
