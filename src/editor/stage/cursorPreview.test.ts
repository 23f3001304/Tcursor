import { describe, it, expect } from "vitest";
import { cursorAt, idAt } from "./cursorPreview";

describe("idAt", () => {
  // The TS mirror of Rust `CursorLayer::id_at` - same "last sample with t <= ms" rule, same
  // "nothing yet" answer before the first sample (where Rust returns None, this returns null).
  const track: [number, number][] = [[100, 0], [400, 1], [900, 0]];

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
  // Unchanged neighbour of idAt, pinned here because the two lookups must stay identical in
  // shape: same binary search, differing only in what "nothing yet" means (arrow vs null).
  it("defaults to arrow before the first sample and on an empty track", () => {
    expect(cursorAt([], 0)).toBe("arrow");
    expect(cursorAt([{ t: 50, kind: "hand" }], 10)).toBe("arrow");
  });
  it("returns the kind in force", () => {
    const kinds = [{ t: 0, kind: "arrow" }, { t: 100, kind: "ibeam" }];
    expect(cursorAt(kinds, 100)).toBe("ibeam");
    expect(cursorAt(kinds, 99)).toBe("arrow");
  });
});
