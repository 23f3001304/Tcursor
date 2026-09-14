import { describe, expect, it } from "vitest";
import { filmstripCount, EDITOR_TRACK_W, FILMSTRIP_COUNT, FILMSTRIP_HEIGHT } from "./filmstripPlan";

describe("filmstripCount", () => {
  it("fills the editor's usual track with tiles at their own aspect, not with tiny repeats", () => {
    // 1336px of track, 80px tall tiles -> 9 tiles ~148px wide, within a few percent of 16:9.
    expect(filmstripCount(EDITOR_TRACK_W, FILMSTRIP_HEIGHT)).toBe(9);
    expect(Math.round(EDITOR_TRACK_W / 9)).toBe(148); // vs 142 for a tile at a true 16:9
  });

  it("asks for more tiles on a wider track and fewer on a narrower one", () => {
    expect(filmstripCount(1920, 80)).toBe(14);
    expect(filmstripCount(900, 80)).toBe(8);
  });

  it("never drops below 8 tiles (Rust's own floor) or climbs past 24", () => {
    expect(filmstripCount(100, 80)).toBe(8);
    expect(filmstripCount(0, 80)).toBe(8);
    expect(filmstripCount(99_999, 80)).toBe(24);
  });

  it("asks for fewer, bigger tiles as the lane gets taller", () => {
    expect(filmstripCount(EDITOR_TRACK_W, 54)).toBe(14); // the lane's old height
    expect(filmstripCount(EDITOR_TRACK_W, 160)).toBe(8);
  });
});

describe("FILMSTRIP_COUNT", () => {
  it("is the pair Rust's thumbs::FILMSTRIP_* mirrors, so both name the same cache dir", () => {
    expect([FILMSTRIP_COUNT, FILMSTRIP_HEIGHT]).toEqual([9, 80]);
  });
});
