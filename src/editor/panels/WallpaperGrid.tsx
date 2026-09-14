// The background picker's tiles and sections, shared by the Wallpapers and Gradient tabs. Every
// tile is a 96x54 PNG rendered in Rust by the EXPORT's own background code (`background_thumbs`),
// so what the grid shows is what the render produces - not a CSS lookalike that drifts from it. A
// tile with no PNG (the decode needs ffmpeg) falls back to its `css`, so it stays selectable.
//
// One collapsible SECTION per group, holding a wrapping grid three tiles to a row. The sideways
// strips this replaced hid most of every group behind a flick, which is what the owner rejected;
// with only the section holding the current wallpaper open, the stack is shorter than the strips
// were and every tile it does show is whole, with its name under it.
import type { BackgroundSettings } from "../../hud/settings/settings";
import { Shimmer } from "../timeline/Shimmer";
import type { BackgroundThumb } from "../../lib/ipc";
import { CategorySection, defaultOpenIndex } from "../controls/Controls";
import { assetFileName } from "./backgroundAsset";
import { BackgroundAssetCard } from "./BackgroundAssetCard";
import { PackTile } from "./PackTile";

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

/** Tiles of one `kind` cut into sections wherever the backend's `group` field changes, so the
 *  ORDER is Rust's and a new group ships by dropping files into `assets/`. */
export function thumbGroups(thumbs: BackgroundThumb[], kind: "mesh" | "gradient"): { name: string; tiles: Tile[] }[] {
  const groups: { name: string; tiles: Tile[] }[] = [];
  for (const t of thumbs.filter((x) => x.kind === kind)) {
    const tile = tilesOf([t], kind)[0];
    const last = groups[groups.length - 1];
    if (last && last.name === t.group) last.tiles.push(tile);
    else groups.push({ name: t.group, tiles: [tile] });
  }
  return groups;
}

/** The wallpaper sections, with `CLASSIC` leading the first one since it is what every pre-library
 *  project already renders. With no thumbnails at all, Classic is still offered on its own rather
 *  than showing an empty tab. */
export function wallpaperGroups(thumbs: BackgroundThumb[]): { name: string; tiles: Tile[] }[] {
  const groups = thumbGroups(thumbs, "mesh");
  if (!groups.length) return [{ name: "Wallpapers", tiles: [CLASSIC] }];
  groups[0].tiles.unshift(CLASSIC);
  return groups;
}

/** One section's tiles as a WRAPPING grid - three to a row at the 320px panel width, never a
 *  horizontal scroller. The tile is the shared `PackTile` (press spring, accent ring, name
 *  captioned under the picture), so a wallpaper and a cursor pack are chosen the same way. */
export function TileGrid({ tiles, selectedId, onSelect }: {
  tiles: Tile[];
  /** `null` = nothing in this section is active (e.g. the user has a custom gradient). */
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="e-tile-grid">
      {tiles.map((t) => (
        <PackTile key={t.id || "(classic)"} selected={t.id === selectedId} title={t.name} label={t.name}
          onPick={() => onSelect(t.id)}>
          <span className="e-tile-img"
            style={t.png ? { backgroundImage: `url(${t.png})` } : { background: t.css }} />
        </PackTile>
      ))}
    </div>
  );
}

/** BackgroundPanel's Wallpapers tab: a section per backend group, then the user's own file as the
 *  last one. Picking a wallpaper leaves `asset` alone, so coming back to "Your file" restores it
 *  without a re-import. */
/** Six shimmer tiles in the library's own grid, shown by both tabs while `backgroundThumbs()` is
 *  still resolving: the shape of what is coming, instead of a lone Classic tile under a
 *  "Wallpapers 1" heading that read as the library being missing. */
export function TileGridSkeleton() {
  return (
    <div className="e-tile-grid" aria-busy="true" aria-label="Loading the library">
      {Array.from({ length: 6 }, (_, i) => <Shimmer key={i} className="e-tile e-tile-skel" />)}
    </div>
  );
}

export function WallpaperTab({ folder, bg, thumbs, setBg }: {
  /** The recording's folder: an imported background lives inside it (`background/<file>`). */
  folder: string;
  bg: BackgroundSettings;
  /** `null` while the thumbnails are still loading. */
  thumbs: BackgroundThumb[] | null;
  setBg: (patch: Partial<BackgroundSettings>) => void;
}) {
  if (thumbs === null) {
    return (
      <div className="e-grp e-secstack">
        <CategorySection id="bg.loading" label="Wallpapers" count={0} defaultOpen><TileGridSkeleton /></CategorySection>
      </div>
    );
  }
  const groups = wallpaperGroups(thumbs);
  const meshId = bg.kind === "mesh" ? bg.mesh ?? null : null;
  const isAsset = bg.kind === "image" || bg.kind === "video";
  const open = defaultOpenIndex([...groups.map((g) => g.tiles.some((t) => t.id === meshId)), isAsset]);
  return (
    <div className="e-grp e-secstack">
      {groups.map((g, i) => (
        <CategorySection key={g.name} id={`bg.${g.name}`} label={g.name} count={g.tiles.length}
          selectedName={g.tiles.find((t) => t.id === meshId)?.name ?? null} defaultOpen={i === open}>
          <TileGrid tiles={g.tiles} selectedId={meshId}
            onSelect={(id) => setBg({ kind: "mesh", mesh: id })} />
        </CategorySection>
      ))}
      <CategorySection id="bg.yourfile" label="Your file" count={bg.asset ? 1 : 0}
        selectedName={isAsset && bg.asset ? assetFileName(bg.asset) : null}
        defaultOpen={open === groups.length}>
        <BackgroundAssetCard folder={folder} asset={bg.asset} kind={bg.kind}
          onPick={(k) => setBg({ kind: k })}
          onImported={(i) => setBg({ kind: i.kind, asset: i.rel_path })}
          onRemoved={() => setBg({ kind: "mesh", asset: null })} />
      </CategorySection>
    </div>
  );
}
