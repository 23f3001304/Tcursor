// The background picker's tile rows, shared by the Wallpapers and Gradient tabs. Every tile is a
// 96x54 PNG rendered in Rust by the EXPORT's own background code (`background_thumbs`), so what
// the row shows is what the render produces - not a CSS lookalike that drifts from it. A tile
// with no PNG (the decode needs ffmpeg) falls back to its `css`, so it stays selectable.
//
// One ROW per group since the usability pass: the 52-tile library as five wrapping grids was
// ~1000px of a 620px panel, and as five strips it is 295px with every tile still one flick away.
import type { BackgroundThumb } from "../../lib/ipc";
import { TileRow } from "../controls/Controls";

/** One tile: `id` is what the caller writes back on select, `css` the fallback paint. */
export interface Tile { id: string; name: string; png: string; css: string }

/** The legacy bundled `bg.jpg` - first tile in the Wallpapers grid, and the only one whose id is
 *  the empty string (which is exactly what `BackgroundSettings.mesh` stores for it). Its `css` is
 *  a flat approximation: the image itself never goes through `background_thumbs` because the
 *  backend treats "no id" as "the legacy path", not as a library entry. */
export const CLASSIC: Tile = { id: "", name: "Classic", png: "", css: "linear-gradient(135deg, #242938, #58406f)" };

/** Thumbnails of one `kind` as tiles, in the order Rust returned them. */
export function tilesOf(thumbs: BackgroundThumb[], kind: "mesh" | "gradient"): Tile[] {
  return thumbs.filter((t) => t.kind === kind).map((t) => ({
    id: t.id,
    name: t.name,
    png: t.png_base64 ? `data:image/png;base64,${t.png_base64}` : "",
    css: "var(--e-soft)",
  }));
}

/** The wallpaper tiles split into the picker's sections, in the order Rust returned them (which
 *  is already Ribbons, Folds, Scenic with each group alphabetical - the backend owns that order,
 *  and new groups appear here without a code change). `CLASSIC` leads the first section, since it
 *  is what every pre-library project already renders. With no thumbnails at all, Classic is still
 *  offered on its own rather than showing an empty tab. */
export function wallpaperGroups(thumbs: BackgroundThumb[]): { name: string; tiles: Tile[] }[] {
  const groups: { name: string; tiles: Tile[] }[] = [];
  for (const t of thumbs.filter((x) => x.kind === "mesh")) {
    const tile = tilesOf([t], "mesh")[0];
    const last = groups[groups.length - 1];
    if (last && last.name === t.group) last.tiles.push(tile);
    else groups.push({ name: t.group, tiles: [tile] });
  }
  if (!groups.length) return [{ name: "Wallpapers", tiles: [CLASSIC] }];
  groups[0].tiles.unshift(CLASSIC);
  return groups;
}

export function WallpaperRow({
  label,
  tiles,
  selectedId,
  onSelect,
}: {
  /** The group's name, which is the row's own label ("Ribbons", "Folds", "Presets"). */
  label: string;
  tiles: Tile[];
  /** `null` = nothing in this row is active (e.g. the user has a custom gradient). */
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <TileRow label={label} ariaLabel={label} selectedId={selectedId} onSelect={onSelect}
      tiles={tiles.map((t) => ({
        id: t.id,
        name: t.name,
        // The thumbnail bleeds to the tile's edges; its name rides in over it on hover, and is
        // also the tile's `title`, so it is never only-on-hover information.
        content: <span className="e-tile-img"
          style={t.png ? { backgroundImage: `url(${t.png})` } : { background: t.css }} />,
      }))} />
  );
}
