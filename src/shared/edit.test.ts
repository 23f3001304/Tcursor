import { describe, expect, it } from "vitest";
import { isMask, MASK_KINDS, resolveTrim, type Caption, type EditDoc, type EffectRegion } from "./edit";
import type { CaptionStyle, Settings } from "../hud/settings/settings";

describe("resolveTrim", () => {
  it("treats out_ms === 0 as no trim yet - the whole clip", () => {
    expect(resolveTrim({ in_ms: 0, out_ms: 0 }, 12_345)).toEqual({ inMs: 0, outMs: 12_345 });
  });

  it("returns the trim range unchanged when it fits inside the clip", () => {
    expect(resolveTrim({ in_ms: 2_000, out_ms: 8_000 }, 10_000)).toEqual({ inMs: 2_000, outMs: 8_000 });
  });

  it("clamps out_ms beyond the real duration down to the clip length", () => {
    expect(resolveTrim({ in_ms: 2_000, out_ms: 999_999 }, 10_000)).toEqual({ inMs: 2_000, outMs: 10_000 });
  });

  it("clamps in_ms to the resolved out_ms instead of going negative-length", () => {
    expect(resolveTrim({ in_ms: 9_000, out_ms: 5_000 }, 10_000)).toEqual({ inMs: 5_000, outMs: 5_000 });
  });
});

describe("the caption track's TS mirror", () => {
  it("types a caption exactly as Rust serializes it", () => {
    const c: Caption = {
      id: "c0",
      start_ms: 1000,
      end_ms: 3000,
      text: "hello world",
      words: [
        { start_ms: 1000, end_ms: 1500, text: "hello" },
        { start_ms: 1500, end_ms: 3000, text: "world" },
      ],
    };
    expect(c.words[1].text).toBe("world");
  });

  it("puts captions on EditDoc next to every other lane", () => {
    const doc = {} as EditDoc;
    const lane: Caption[] = doc.captions ?? [];
    expect(lane).toEqual([]);
  });

  it("keeps the hotkey toggle and the caption style as two different fields", () => {
    const s = {} as Settings;
    const style: CaptionStyle | undefined = s.captions;
    const hotkeyToggle: boolean | undefined = s.clickfx?.captions;
    expect(style).toBeUndefined();
    expect(hotkeyToggle).toBeUndefined();
  });
});

describe("mask kinds", () => {
  const spot: EffectRegion = {
    id: "e0",
    kind: "spotlight",
    start_ms: 0,
    end_ms: 1,
    fade_in_ms: 250,
    fade_out_ms: 250,
    layer: 0,
  };
  it("names the three mask kinds and nothing else", () => {
    expect([...MASK_KINDS]).toEqual(["blur", "pixelate", "highlight"]);
  });
  it("isMask is false for a spotlight and true for every mask kind", () => {
    expect(isMask(spot)).toBe(false);
    for (const kind of MASK_KINDS) expect(isMask({ ...spot, kind })).toBe(true);
  });
});
