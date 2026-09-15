import { describe, expect, it } from "vitest";
import type { Caption } from "../../../shared/edit";
import type { CaptionStyle } from "../../../hud/settings/settings";
import { captionAt, easeOut, layoutCaption, wordSpan, wrapLines } from "./captionPreview";
import type { CaptionAnim } from "../../../hud/settings/settings";

const STYLE: CaptionStyle = {
  enabled: true,
  position: "bottom",
  size: "m",
  pill: true,
  highlight: true,
  model: "base.en",
  language: "en",
  font_pct: 0,
  text_color: [255, 255, 255],
  highlight_color: null,
  pill_color: [0, 0, 0],
  pill_alpha: 62,
  animation: "fade",
  animation_ms: 120,
};

const helloWorld = (): Caption => ({
  id: "c0",
  start_ms: 1000,
  end_ms: 3000,
  text: "hello world",
  words: [
    { start_ms: 1000, end_ms: 1500, text: "hello" },
    { start_ms: 1500, end_ms: 3000, text: "world" },
  ],
});

const near = (a: number, b: number, what: string) => expect(Math.abs(a - b), what).toBeLessThan(1e-3);

describe("the caption preview mirrors the export's layout exactly", () => {
  it("case A: bottom, medium, one line at 1920x1080", () => {
    const l = layoutCaption(helloWorld(), STYLE, 1920, 1080, 2000);
    expect(l.lines).toEqual(["hello world"]);
    near(l.fontPx, 41.04, "fontPx");
    near(l.lineH, 54.1728, "lineH");
    near(l.pill[0], 818.0016, "pill x");
    near(l.pill[1], 916.92, "pill y");
    near(l.pill[2], 283.9968, "pill w");
    near(l.pill[3], 82.08, "pill h");
    expect(l.baselines.length).toBe(1);
    near(l.baselines[0], 971.9136, "baseline 0");
    expect(l.hi).toBe(1);
    near(l.alpha, 1, "alpha");
  });

  it("case B: top, small, two lines, mid fade at 1280x720", () => {
    const style: CaptionStyle = { ...STYLE, position: "top", size: "s", highlight: false };
    const c: Caption = {
      id: "c0",
      start_ms: 0,
      end_ms: 4000,
      text: "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii",
      words: [],
    };
    const l = layoutCaption(c, style, 1280, 720, 60);
    expect(l.lines).toEqual(["aaaa bbbb cccc dddd eeee ffff gggg hhhh", "iiii"]);
    near(l.fontPx, 21.6, "fontPx");
    near(l.lineH, 28.512, "lineH");
    near(l.pill[0], 408.016, "pill x");
    near(l.pill[1], 54, "pill y");
    near(l.pill[2], 463.968, "pill w");
    near(l.pill[3], 71.712, "pill h");
    near(l.baselines[0], 82.944, "baseline 0");
    near(l.baselines[1], 111.456, "baseline 1");
    expect(l.hi).toBeNull();
    near(l.alpha, 0.5, "alpha");
  });

  it("fades out symmetrically and shows nothing outside the span", () => {
    const c = helloWorld();
    near(layoutCaption(c, STYLE, 1920, 1080, 2940).alpha, 0.5, "fade out");
    near(layoutCaption(c, STYLE, 1920, 1080, 1000).alpha, 0, "at the start");
    near(layoutCaption(c, STYLE, 1920, 1080, 3000).alpha, 0, "at the end");
  });

  it("picks the same caption the export would, with an exclusive end", () => {
    const caps: Caption[] = [
      helloWorld(),
      { id: "c1", start_ms: 4000, end_ms: 5000, text: "next", words: [] },
    ];
    expect(captionAt(caps, 2000)?.id).toBe("c0");
    expect(captionAt(caps, 3500)).toBeNull();
    expect(captionAt(caps, 3000)).toBeNull();
    expect(captionAt(caps, 4999)?.id).toBe("c1");
    expect(captionAt([], 0)).toBeNull();
  });

  it("lays an empty caption out to nothing", () => {
    const c: Caption = { id: "c0", start_ms: 0, end_ms: 1000, text: "   ", words: [] };
    const l = layoutCaption(c, STYLE, 1920, 1080, 500);
    expect(l.lines).toEqual([]);
    near(l.pill[2], 0, "pill w");
    near(l.pill[3], 0, "pill h");
  });

  it("tracks the word being spoken and holds the last one through a pause", () => {
    expect(layoutCaption(helloWorld(), STYLE, 1920, 1080, 1100).hi).toBe(0);
    expect(layoutCaption(helloWorld(), STYLE, 1920, 1080, 1500).hi).toBe(1);
    expect(layoutCaption({ ...helloWorld(), start_ms: 0 }, STYLE, 1920, 1080, 500).hi).toBeNull();
  });

  it("wraps on words and never drops one", () => {
    expect(wrapLines("", 42)).toEqual([]);
    expect(wrapLines("one two", 42)).toEqual(["one two"]);
    expect(wrapLines("aaaa bbbb cc", 9)).toEqual(["aaaa bbbb", "cc"]);
    expect(wrapLines("supercalifragilistic ok", 8)).toEqual(["supercalifragilistic", "ok"]);
  });
});

// (anim, tMs, alpha, rise, scale, wordsShown with -1 for none, wordAlpha).
// The same 20 rows are PARITY in captionlayout_tests.rs; either side drifting is a red test.
// Subject: helloWorld (1000..3000, words at 1000 and 1500) at 1920x1080, animation_ms 120.
const PARITY: [CaptionAnim, number, number, number, number, number, number][] = [
  ["none", 1060, 1, 0, 1, -1, 1],
  ["none", 2000, 1, 0, 1, -1, 1],
  ["none", 2940, 1, 0, 1, -1, 1],
  ["none", 3100, 0, 0, 1, -1, 1],
  ["fade", 1060, 0.5, 0, 1, -1, 1],
  ["fade", 2000, 1, 0, 1, -1, 1],
  ["fade", 2940, 0.5, 0, 1, -1, 1],
  ["fade", 3100, 0, 0, 1, -1, 1],
  ["rise", 1060, 0.5, 3.3858, 1, -1, 1],
  ["rise", 2000, 1, 0, 1, -1, 1],
  ["rise", 2940, 0.5, 0, 1, -1, 1],
  ["rise", 3100, 0, 0, 1, -1, 1],
  ["pop", 1060, 0.5, 0, 0.99, -1, 1],
  ["pop", 2000, 1, 0, 1, -1, 1],
  ["pop", 2940, 0.5, 0, 1, -1, 1],
  ["pop", 3100, 0, 0, 1, -1, 1],
  ["words", 1060, 0.5, 0, 1, 1, 0.5],
  ["words", 2000, 1, 0, 1, 2, 1],
  ["words", 2940, 0.5, 0, 1, 2, 1],
  ["words", 3100, 0, 0, 1, 2, 1],
];

describe("the caption preview animates exactly as the export does", () => {
  it.each(PARITY)("%s at %i ms", (animation, t, alpha, rise, scale, shown, wordAlpha) => {
    const l = layoutCaption(helloWorld(), { ...STYLE, animation }, 1920, 1080, t);
    near(l.alpha, alpha, "alpha");
    near(l.rise, rise, "rise");
    near(l.scale, scale, "scale");
    near(l.wordAlpha, wordAlpha, "word alpha");
    expect(l.wordsShown).toBe(shown >= 0 ? shown : null);
    near(l.fontPx, 41.04 * scale, "font px");
    near(l.pill[1], 999 - 82.08 * scale + rise, "pill y");
  });

  it("moves a rising pill and its baselines together", () => {
    const l = layoutCaption(helloWorld(), { ...STYLE, animation: "rise" }, 1920, 1080, 1060);
    near(l.pill[1], 916.92 + 3.3858, "pill y");
    near(l.baselines[0], 971.9136 + 3.3858, "baseline 0");
  });

  it("re-lays a popping pill out around the scaled font", () => {
    const l = layoutCaption(helloWorld(), { ...STYLE, animation: "pop" }, 1920, 1080, 1000);
    near(l.scale, 0.92, "scale");
    near(l.fontPx, 41.04 * 0.92, "font px");
    near(l.pill[2], 283.9968 * 0.92, "pill w");
    near(l.pill[3], 82.08 * 0.92, "pill h");
  });

  it("falls back to the fade when a words caption has no word timings", () => {
    const c: Caption = { ...helloWorld(), words: [] };
    const l = layoutCaption(c, { ...STYLE, animation: "words" }, 1920, 1080, 1060);
    expect(l.wordsShown).toBeNull();
    near(l.alpha, 0.5, "the plain fade");
  });

  it("cuts in at full alpha when the animation is zero length", () => {
    for (const animation of ["fade", "rise", "pop", "words"] as CaptionAnim[]) {
      const s = { ...STYLE, animation, animation_ms: 0 };
      const l = layoutCaption(helloWorld(), s, 1920, 1080, 1000);
      near(l.alpha, 1, "alpha");
      near(l.rise, 0, "rise");
      near(l.scale, 1, "scale");
      near(l.wordAlpha, 1, "word alpha");
    }
  });

  it("lets a fine font percent override the size rung, clamped", () => {
    const fine = (font_pct: number) =>
      layoutCaption(helloWorld(), { ...STYLE, font_pct }, 1920, 1080, 2000).fontPx;
    near(fine(0), 41.04, "0 keeps the M rung");
    near(fine(5), 54, "5% of 1080");
    near(fine(0.5), 1080 * 0.015, "clamped up");
    near(fine(40), 1080 * 0.08, "clamped down");
  });

  it("walks word spans by character offset and eases the same cubic", () => {
    const c = helloWorld();
    expect(wordSpan(c, 0)).toEqual([0, 5]);
    expect(wordSpan(c, 1)).toEqual([6, 11]);
    expect(wordSpan(c, 2)).toBeNull();
    near(easeOut(0), 0, "start");
    near(easeOut(0.5), 0.875, "midpoint");
    near(easeOut(1), 1, "end");
    near(easeOut(-1), 0, "clamped below");
    near(easeOut(2), 1, "clamped above");
  });
});
