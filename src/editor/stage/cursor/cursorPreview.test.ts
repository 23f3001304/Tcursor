// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { cursorAt, idAt } from "./cursorPreview";
import {
  backBox,
  cursorMorphAt,
  lerpBox,
  morphEase,
  spriteBox,
  BACK_SCALE,
  GLASS_ALPHA,
  MORPH_MS,
  PILL_W,
} from "./cursorGlass";

describe("idAt", () => {
  const track: [number, number][] = [
    [100, 0],
    [400, 1],
    [900, 0],
  ];

  it("has no cursor before the first sample", () => {
    expect(idAt(track, 0)).toBe(null);
    expect(idAt(track, 99)).toBe(null);
  });
  it("takes the new sample exactly on its boundary", () => {
    expect(idAt(track, 100)).toBe(0);
    expect(idAt(track, 400)).toBe(1);
  });
  it("holds the sample in force until the next one", () => {
    expect(idAt(track, 399)).toBe(0);
    expect(idAt(track, 899)).toBe(1);
  });
  it("keeps the last sample in force forever after it", () => {
    expect(idAt(track, 900)).toBe(0);
    expect(idAt(track, 10_000_000)).toBe(0);
  });
  it("has no cursor at all on an empty track", () => {
    expect(idAt([], 0)).toBe(null);
    expect(idAt([], 5000)).toBe(null);
  });
  it("distinguishes id 0 from 'no sample' (0 is a real id, not a falsy miss)", () => {
    expect(idAt([[0, 0]], 0)).toBe(0);
    expect(idAt([[10, 0]], 0)).toBe(null);
  });
});

describe("cursorAt", () => {
  it("defaults to arrow before the first sample and on an empty track", () => {
    expect(cursorAt([], 0)).toBe("arrow");
    expect(cursorAt([{ t: 50, kind: "hand" }], 10)).toBe("arrow");
  });
  it("returns the kind in force", () => {
    const kinds = [
      { t: 0, kind: "arrow" },
      { t: 100, kind: "ibeam" },
    ];
    expect(cursorAt(kinds, 100)).toBe("ibeam");
    expect(cursorAt(kinds, 99)).toBe("arrow");
  });
});

describe("the glass cursor material's preview mirror", () => {
  it("pins the constants the export owns", () => {
    expect(GLASS_ALPHA).toBe(0.65);
    expect(BACK_SCALE).toBe(2.2);
    expect(PILL_W).toBe(0.35);
  });
  it("is a disc for every shape but the I-beam, which is a horizontal pill", () => {
    const disc = backBox("arrow", 40);
    expect(disc.ry).toBeCloseTo(44, 6);
    expect(disc.rx).toBeCloseTo(disc.ry, 6);
    for (const k of ["hand", "move", "busy", "resize_ns"]) {
      expect(backBox(k, 40).ry).toBeCloseTo(disc.ry, 6);
    }
    const pill = backBox("ibeam", 40);
    expect(pill.rx).toBeCloseTo(disc.rx, 6);
    expect(pill.ry).toBeCloseTo(disc.rx * 0.35, 6);
  });
});

describe("cursorMorphAt", () => {
  const kinds = [
    { t: 0, kind: "arrow" },
    { t: 1000, kind: "ibeam" },
  ];
  it("is a settled arrow before the first sample and on an empty track", () => {
    expect(cursorMorphAt([], 500)).toEqual({ kind: "arrow", prev: "arrow", m: 1 });
    expect(cursorMorphAt(kinds, 500)).toEqual({ kind: "arrow", prev: "arrow", m: 1 });
  });
  it("starts a change at the previous shape and settles after MORPH_MS", () => {
    const at0 = cursorMorphAt(kinds, 1000);
    expect([at0.kind, at0.prev]).toEqual(["ibeam", "arrow"]);
    expect(at0.m).toBeCloseTo(0, 6);
    expect(cursorMorphAt(kinds, 1000 + MORPH_MS).m).toBeCloseTo(1, 6);
    expect(cursorMorphAt(kinds, 9999).m).toBeCloseTo(1, 6);
  });
  it("eases out like the click effects", () => {
    for (const [p, want] of [
      [0, 0],
      [0.25, 0.578125],
      [0.5, 0.875],
      [0.75, 0.984375],
      [1, 1],
    ]) {
      expect(morphEase(p)).toBeCloseTo(want, 6);
    }
    expect(morphEase(-1)).toBe(0);
    expect(morphEase(2)).toBe(1);
  });
});

describe("the glass state cross-fade's box", () => {
  const img = { naturalWidth: 20, naturalHeight: 10 } as HTMLImageElement;
  it("puts the hotspot on the cursor point", () => {
    expect(spriteBox(img, [0.5, 0.5], 10, [100, 100], 10)).toEqual([90, 95, 20, 10]);
  });
  it("interpolates between the two states without drifting off the cursor", () => {
    const a = spriteBox(img, [0.5, 0.5], 10, [100, 100], 10);
    const b = spriteBox(
      { naturalWidth: 30, naturalHeight: 10 } as HTMLImageElement,
      [0.5, 0.5],
      10,
      [100, 100],
      20,
    );
    for (const m of [0, 0.25, 0.5, 0.75, 1]) {
      const box = lerpBox(a, b, m);
      expect(box[0] + box[2] / 2).toBeCloseTo(100, 6);
      expect(box[1] + box[3] / 2).toBeCloseTo(100, 6);
    }
    expect(lerpBox(a, b, 0)).toEqual(a);
    expect(lerpBox(a, b, 1)).toEqual(b);
    expect(lerpBox(a, b, -1)).toEqual(a);
    expect(lerpBox(a, b, 2)).toEqual(b);
  });
});
