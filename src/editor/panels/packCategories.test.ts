import { describe, it, expect } from "vitest";
import type { CursorPackInfo } from "../../lib/ipc";
import { CATEGORY_ORDER, packCategories } from "./packCategories";

const pack = (id: string, category: string, builtin = true): CursorPackInfo =>
  ({ id, name: id, category, builtin, dir: `C:\\packs\\${id}`, files: {}, busy: null, material: null });

describe("packCategories", () => {
  it("groups by the pack's own category, in the curated order, never by builtin", () => {
    const cats = packCategories([
      pack("pixel", "Retro"), pack("cat", "Playful"), pack("fluent", "Classic"),
      pack("neon", "Glass and glow"), pack("ink", "Drawn"), pack("mine", "Imported", false),
    ]);
    expect(cats.map((c) => c.name)).toEqual(CATEGORY_ORDER);
    expect(cats[0].packs.map((p) => p.id)).toEqual(["fluent"]);
    expect(cats[5].packs.map((p) => p.id)).toEqual(["mine"]);
  });

  it("keeps the backend's order inside a section", () => {
    const cats = packCategories([pack("cat", "Playful"), pack("clay", "Playful"), pack("glove", "Playful")]);
    expect(cats[0].packs.map((p) => p.id)).toEqual(["cat", "clay", "glove"]);
  });

  it("omits a category nothing is in, rather than showing an empty heading", () => {
    const cats = packCategories([pack("fluent", "Classic")]);
    expect(cats.map((c) => c.name)).toEqual(["Classic"]);
  });

  it("puts an unknown category after the curated ones, alphabetically", () => {
    const cats = packCategories([
      pack("zeta", "Zebra"), pack("aero", "Glass and glow"), pack("alpha", "Aardvark"),
    ]);
    expect(cats.map((c) => c.name)).toEqual(["Glass and glow", "Aardvark", "Zebra"]);
  });

  it("lists a pack with a missing or blank category under Imported", () => {
    // Rust already applies this fallback; repeating it here only keeps a hand-edited or older
    // reply from producing a section with no name at all.
    const blank = { ...pack("odd", ""), category: "  " };
    const { category: _gone, ...absent } = pack("older", "");
    const cats = packCategories([blank, absent as CursorPackInfo]);
    expect(cats.map((c) => c.name)).toEqual(["Imported"]);
    expect(cats[0].packs.map((p) => p.id)).toEqual(["odd", "older"]);
  });
});
