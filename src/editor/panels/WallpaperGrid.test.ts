import { describe, it, expect } from "vitest";
import type { BackgroundThumb } from "../../lib/ipc";
import type { BackgroundSettings } from "../../hud/settings/settings";
import { CLASSIC, tilesOf, wallpaperGroups } from "./WallpaperGrid";
import { isPreset } from "./GradientTab";

const thumbs: BackgroundThumb[] = [
  { id: "brand", name: "Brand", kind: "mesh", group: "Ribbons", png_base64: "AAA" },
  { id: "paper", name: "Paper", kind: "mesh", group: "Ribbons", png_base64: "" },
  { id: "folds-citrus", name: "Citrus", kind: "mesh", group: "Folds", png_base64: "CCC" },
  { id: "dusk", name: "Dusk", kind: "gradient", group: "Gradients", png_base64: "BBB",
    gradient: { from: [1, 2, 3], mid: [4, 5, 6], to: [7, 8, 9], angle_deg: 160 } },
];

const bg = (patch: Partial<BackgroundSettings>): BackgroundSettings => ({
  kind: "gradient", solid: [0, 0, 0], mesh: "", gradient_from: [1, 2, 3], gradient_mid: [4, 5, 6],
  gradient_to: [7, 8, 9], gradient_angle_deg: 160, blur: 0, dim: 0, ...patch,
});

describe("tilesOf", () => {
  it("keeps only the asked-for kind, in Rust's order, as data URLs", () => {
    const mesh = tilesOf(thumbs, "mesh");
    expect(mesh.map((t) => t.id)).toEqual(["brand", "paper", "folds-citrus"]);
    expect(mesh[0].png).toBe("data:image/png;base64,AAA");
    expect(tilesOf(thumbs, "gradient").map((t) => t.id)).toEqual(["dusk"]);
  });

  it("leaves `png` empty when the backend could not render one, so the tile falls back to css", () => {
    // The wallpaper decode needs ffmpeg; a missing thumbnail must not make the wallpaper itself
    // unselectable, which is why the grid paints `css` instead of hiding the tile.
    const paper = tilesOf(thumbs, "mesh")[1];
    expect(paper.png).toBe("");
    expect(paper.css).not.toBe("");
  });

  it("gives the legacy mesh the empty id BackgroundSettings.mesh actually stores for it", () => {
    expect(CLASSIC.id).toBe("");
    expect(CLASSIC.name).toBe("Classic");
  });
});

describe("wallpaperGroups", () => {
  it("splits the wallpapers into the backend's own sections, in its order", () => {
    const groups = wallpaperGroups(thumbs);
    expect(groups.map((g) => g.name)).toEqual(["Ribbons", "Folds"]);
    expect(groups[1].tiles.map((t) => t.id)).toEqual(["folds-citrus"]);
  });

  it("puts Classic first in the first section, never in a later one", () => {
    const groups = wallpaperGroups(thumbs);
    expect(groups[0].tiles[0].id).toBe(CLASSIC.id);
    expect(groups[1].tiles.some((t) => t.id === CLASSIC.id)).toBe(false);
  });

  it("still offers Classic when no thumbnails came back at all", () => {
    // The wallpaper decode needs ffmpeg, and the fetch can reject outright - the tab must not be
    // empty in either case, because Classic renders through a path that never needed the list.
    expect(wallpaperGroups([])).toEqual([{ name: "Wallpapers", tiles: [CLASSIC] }]);
  });
});

describe("isPreset", () => {
  const dusk = thumbs[3];
  it("matches only when every stop AND the angle agree", () => {
    expect(isPreset(dusk, bg({}))).toBe(true);
    expect(isPreset(dusk, bg({ gradient_angle_deg: 161 }))).toBe(false);
    expect(isPreset(dusk, bg({ gradient_to: [7, 8, 10] }))).toBe(false);
  });

  it("treats a removed middle stop as a difference, whether it is null or absent", () => {
    expect(isPreset(dusk, bg({ gradient_mid: null }))).toBe(false);
    const { gradient_mid: _drop, ...noMid } = bg({});
    expect(isPreset(dusk, noMid as BackgroundSettings)).toBe(false);
    const twoStop: BackgroundThumb = { ...dusk, gradient: { ...dusk.gradient!, mid: null } };
    expect(isPreset(twoStop, bg({ gradient_mid: null }))).toBe(true);
    expect(isPreset(twoStop, noMid as BackgroundSettings)).toBe(true);
  });

  it("never matches a wallpaper tile, which carries no stops at all", () => {
    expect(isPreset(thumbs[0], bg({}))).toBe(false);
  });
});
