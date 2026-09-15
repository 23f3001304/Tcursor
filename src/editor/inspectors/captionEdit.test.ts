import { describe, expect, it } from "vitest";
import type { Caption, CaptionWord } from "../../shared/edit";
import { insideSpan, nextCaption, splitPoints, wordsSurvive } from "./captionEdit";

const w = (start_ms: number, end_ms: number, text: string): CaptionWord => ({ start_ms, end_ms, text });
const cap = (over: Partial<Caption> = {}): Caption => ({
  id: "c0",
  start_ms: 1000,
  end_ms: 3000,
  text: "hello there world",
  words: [w(1000, 1400, "hello"), w(1400, 2000, "there"), w(2000, 3000, "world")],
  ...over,
});

describe("splitPoints", () => {
  it("offers a split before every word but the first, at that word's own start", () => {
    expect(splitPoints(cap())).toEqual([
      { atMs: 1400, label: "there" },
      { atMs: 2000, label: "world" },
    ]);
  });

  it("offers nothing on a one-word caption, because there is no boundary inside it", () => {
    expect(splitPoints(cap({ text: "hello", words: [w(1000, 3000, "hello")] }))).toEqual([]);
  });

  it("offers nothing on a caption with no word timings - the playhead is the only cut there", () => {
    expect(splitPoints(cap({ words: [] }))).toEqual([]);
  });

  it("drops a boundary that is not strictly inside the caption, which Rust would reject anyway", () => {
    const c = cap({ words: [w(1000, 1400, "hello"), w(1000, 2000, "there"), w(3000, 3200, "late")] });
    expect(splitPoints(c)).toEqual([]);
  });

  it("never offers the same instant twice", () => {
    const c = cap({ text: "a b c", words: [w(1000, 1500, "a"), w(1500, 1500, "b"), w(1500, 3000, "c")] });
    expect(splitPoints(c)).toEqual([{ atMs: 1500, label: "b" }]);
  });
});

describe("wordsSurvive", () => {
  it("is true only for the text the word timings already spell, so the highlight stays honest", () => {
    expect(wordsSurvive(cap(), "hello there world")).toBe(true);
    expect(wordsSurvive(cap(), "  hello there world  ")).toBe(true);
    expect(wordsSurvive(cap(), "hello there World")).toBe(false);
    expect(wordsSurvive(cap(), "completely different copy")).toBe(false);
  });

  it("is false when there are no word timings to lose in the first place", () => {
    expect(wordsSurvive(cap({ words: [] }), "hello there world")).toBe(false);
  });
});

describe("nextCaption", () => {
  const a = cap();
  const b = cap({ id: "c1", start_ms: 3200, end_ms: 5000, text: "and again", words: [] });

  it("is the caption that starts after this one, in time order not array order", () => {
    expect(nextCaption([b, a], "c0")?.id).toBe("c1");
  });

  it("is null on the last caption, which is exactly when Merge must be inert", () => {
    expect(nextCaption([a, b], "c1")).toBeNull();
    expect(nextCaption([a, b], "nope")).toBeNull();
  });
});

describe("insideSpan", () => {
  it("is true only strictly between the edges, matching Rust's split rule", () => {
    expect(insideSpan(cap(), 2000)).toBe(true);
    expect(insideSpan(cap(), 1000)).toBe(false);
    expect(insideSpan(cap(), 3000)).toBe(false);
    expect(insideSpan(cap(), 9000)).toBe(false);
  });
});
