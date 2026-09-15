import type { BackgroundSettings } from "../../../hud/settings/settings";
import { Shimmer } from "../../timeline/lanes/Shimmer";
import type { BackgroundThumb } from "../../../shared/ipc";
import { CategorySection, defaultOpenIndex } from "../../controls/Controls";
import { assetFileName } from "./backgroundAsset";
import { BackgroundAssetCard } from "./BackgroundAssetCard";
import { PackTile } from "../PackTile";

export interface Tile {
  id: string;
  name: string;
  png: string;
  css: string;
}

export const CLASSIC: Tile = {
  id: "",
  name: "Classic",
  png: "",
  css: "linear-gradient(135deg, #242938, #58406f)",
};

export function tilesOf(thumbs: BackgroundThumb[], kind: "mesh" | "gradient"): Tile[] {
  return thumbs
    .filter((t) => t.kind === kind)
    .map((t) => ({
      id: t.id,
      name: t.name,
      png: t.png_base64 ? `data:image/png;base64,${t.png_base64}` : "",
      css: "var(--e-soft)",
    }));
}

export function thumbGroups(
  thumbs: BackgroundThumb[],
  kind: "mesh" | "gradient",
): { name: string; tiles: Tile[] }[] {
  const groups: { name: string; tiles: Tile[] }[] = [];
  for (const t of thumbs.filter((x) => x.kind === kind)) {
    const tile = tilesOf([t], kind)[0];
    const last = groups[groups.length - 1];
    if (last && last.name === t.group) last.tiles.push(tile);
    else groups.push({ name: t.group, tiles: [tile] });
  }
  return groups;
}

export function wallpaperGroups(thumbs: BackgroundThumb[]): { name: string; tiles: Tile[] }[] {
  const groups = thumbGroups(thumbs, "mesh");
  if (!groups.length) return [{ name: "Wallpapers", tiles: [CLASSIC] }];
  groups[0].tiles.unshift(CLASSIC);
  return groups;
}

export function TileGrid({
  tiles,
  selectedId,
  onSelect,
}: {
  tiles: Tile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <div className="e-tile-grid">
      {tiles.map((t) => (
        <PackTile
          key={t.id || "(classic)"}
          selected={t.id === selectedId}
          title={t.name}
          label={t.name}
          onPick={() => onSelect(t.id)}
        >
          <span
            className="e-tile-img"
            style={t.png ? { backgroundImage: `url(${t.png})` } : { background: t.css }}
          />
        </PackTile>
      ))}
    </div>
  );
}

export function TileGridSkeleton() {
  return (
    <div className="e-tile-grid" aria-busy="true" aria-label="Loading the library">
      {Array.from({ length: 6 }, (_, i) => (
        <Shimmer key={i} className="e-tile e-tile-skel" />
      ))}
    </div>
  );
}

export function WallpaperTab({
  folder,
  bg,
  thumbs,
  setBg,
}: {
  folder: string;
  bg: BackgroundSettings;
  thumbs: BackgroundThumb[] | null;
  setBg: (patch: Partial<BackgroundSettings>) => void;
}) {
  if (thumbs === null) {
    return (
      <div className="e-grp e-secstack">
        <CategorySection id="bg.loading" label="Wallpapers" count={0} defaultOpen>
          <TileGridSkeleton />
        </CategorySection>
      </div>
    );
  }
  const groups = wallpaperGroups(thumbs);
  const meshId = bg.kind === "mesh" ? (bg.mesh ?? null) : null;
  const isAsset = bg.kind === "image" || bg.kind === "video";
  const open = defaultOpenIndex([...groups.map((g) => g.tiles.some((t) => t.id === meshId)), isAsset]);
  return (
    <div className="e-grp e-secstack">
      {groups.map((g, i) => (
        <CategorySection
          key={g.name}
          id={`bg.${g.name}`}
          label={g.name}
          count={g.tiles.length}
          selectedName={g.tiles.find((t) => t.id === meshId)?.name ?? null}
          defaultOpen={i === open}
        >
          <TileGrid
            tiles={g.tiles}
            selectedId={meshId}
            onSelect={(id) => setBg({ kind: "mesh", mesh: id })}
          />
        </CategorySection>
      ))}
      <CategorySection
        id="bg.yourfile"
        label="Your file"
        count={bg.asset ? 1 : 0}
        selectedName={isAsset && bg.asset ? assetFileName(bg.asset) : null}
        defaultOpen={open === groups.length}
      >
        <BackgroundAssetCard
          folder={folder}
          asset={bg.asset}
          kind={bg.kind}
          onPick={(k) => setBg({ kind: k })}
          onImported={(i) => setBg({ kind: i.kind, asset: i.rel_path })}
          onRemoved={() => setBg({ kind: "mesh", asset: null })}
        />
      </CategorySection>
    </div>
  );
}
