# src/editor/panels/WallpaperGrid.tsx

The background picker's tile rows, shared by `BackgroundPanel`'s Wallpapers tab and `GradientTab`'s preset row. Every tile is a 96x54 PNG rendered in Rust by the export's OWN background code (`background_thumbs`), so what the row shows is what the render produces - not a CSS lookalike that drifts from it as the two are edited apart. That is also why the old `GRADIENT_PRESETS` CSS strings in `backgroundPresets.ts` were removed rather than reused here.

**The file is called `WallpaperGrid.tsx` and it is a grid again.** The usability pass had turned each group into a horizontally scrolling strip; the arrangements pass (2026-09-14) put the wrapping grid back, inside a collapsible `CategorySection` per group, on the owner's ruling that nothing in a panel may move sideways and that a tile's name belongs under the tile rather than in a tooltip the strip then clips. The data helpers below - which are what most callers actually import - did not change with either pass.

## Tile

```ts
export interface Tile { id: string; name: string; png: string; css: string }
```

One tile's display data.

- `id` - what the caller gets back from `onSelect` (and, for wallpapers, exactly what `BackgroundSettings.mesh` stores).
- `name` - the tile's `title` AND the caption printed under its artwork, always visible. *Why a caption at all:* several wallpapers are deliberately dark and calm, and are hard to tell apart from the picture alone. It used to ride in over the artwork on hover; a name you have to hover to read is what the arrangements pass removed.
- `png` - a `data:image/png;base64,...` URL, or empty.
- `css` - what to paint when `png` is empty.

## CLASSIC

```ts
export const CLASSIC: Tile
```

The legacy bundled `bg.jpg`, first tile in the Wallpapers grid and the only one whose `id` is the empty string - which is exactly what `BackgroundSettings.mesh` stores for it.

Its `css` is a flat approximation rather than a real thumbnail: the backend treats "no id" as "the legacy path", not as a library entry, so the image never comes through `background_thumbs`. *Why keep it first rather than migrate it into the library:* every project saved before the library renders with it, so it has to stay one click away, and giving it a library id would change what those projects' settings mean.

## tilesOf

```ts
export function tilesOf(thumbs: BackgroundThumb[], kind: "mesh" | "gradient"): Tile[]
```

Thumbnails of one `kind` as tiles, in the order Rust returned them (which is the order the panel shows). A thumb with an empty `png_base64` - the wallpaper decode needs ffmpeg - yields an empty `png` and falls back to `css`, so a missing thumbnail never makes a working wallpaper unselectable.

### Behaviors

- `keeps only the asked-for kind, in Rust's order, as data URLs`.
- `leaves \`png\` empty when the backend could not render one, so the tile falls back to css`.
- `gives the legacy mesh the empty id BackgroundSettings.mesh actually stores for it`.

## thumbGroups

```ts
export function thumbGroups(thumbs: BackgroundThumb[], kind: "mesh" | "gradient"):
  { name: string; tiles: Tile[] }[]
```

Tiles of one `kind` cut into sections wherever the backend's `group` field changes, so the ORDER is whatever Rust emitted (`Ribbons`, `Folds`, `Gradients`, `Metal`, `Scenic`, each alphabetical) - a new group ships by dropping files into `assets/`, with no change here or in either tab.

Split out of `wallpaperGroups` by the arrangements pass so the Gradient tab can build sections the same way. The two kinds can never share a section, because the filter runs before the cut.

### Behaviors

- `cuts one kind into the backend's own sections, in its order`.
- `gives the gradient presets their own section, never a wallpaper one`.
- `has no sections at all when nothing of that kind came back`.

## wallpaperGroups

```ts
export function wallpaperGroups(thumbs: BackgroundThumb[]): { name: string; tiles: Tile[] }[]
```

`thumbGroups(thumbs, "mesh")` with the legacy tile in front: `CLASSIC` is unshifted onto the FIRST group, so the legacy background stays one click away without needing a section of its own. With no thumbnails at all - no ffmpeg, or the fetch rejected - the function returns a single `Wallpapers` group holding just Classic, because Classic renders through a path that never needed the thumbnail list and the tab must not come up empty.

### Behaviors

- `splits the wallpapers into the backend's own sections, in its order`.
- `puts Classic first in the first section, never in a later one`.
- `still offers Classic when no thumbnails came back at all`.
- `renders one collapsible section per group, named and counted, with Your file last` (`BackgroundPanel.test.tsx`).

## TileGrid

```tsx
export function TileGrid({ tiles, selectedId, onSelect }: {
  tiles: Tile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element
```

One section's tiles as a WRAPPING grid - `.e-tile-grid`, three to a row at the 320px panel width, with no `overflow-x` anywhere in it.

Each tile is the shared `PackTile` (`PackTile.md`): the same press spring, the same 2px inset accent ring, the same name captioned under the picture that a cursor pack gets. The face is a `.e-tile-img` span painted with the thumbnail, or with `Tile.css` when the backend could not render one. So a wallpaper and a cursor pack are chosen by the same control, in the same idiom, in two different panels.

**The arithmetic it replaced.** Three-up in a 288px content column was already the panel pass's layout, and the 52-tile library cost about 1000px of a 620px panel - which is what the sideways strips were for. Sections solve the same problem without moving anything: five closed headers at 28px plus the one open group is well under the budget, and the group the user is actually in is the one on screen.

- `selectedId` - `null` means nothing in THIS section is active, which is the normal state for every wallpaper section while the background is a gradient or a solid, and for the gradient section once the user has edited a preset's stops. Compared against `Tile.id`, so the Classic tile is selected exactly when `mesh` is empty.
- `onSelect` - handed the tile's id; the caller decides what that means (a `mesh` id, or a preset to expand into gradient stops).

## TileGridSkeleton

```ts
export function TileGridSkeleton(): JSX.Element
```

Six `Shimmer` tiles in the library's own `.e-tile-grid`, shown by `WallpaperTab` and `GradientTab` inside a single open `CategorySection` while `backgroundThumbs()` is still resolving (`thumbs === null`): the shape of what is coming, instead of a lone Classic tile under a "Wallpapers 1" heading, which read as the library being missing (owner, 2026-09-14). The backend now caches the set on disk and warms it at startup, so this is normally gone before it is seen.

## WallpaperTab

```tsx
export function WallpaperTab({ folder, bg, thumbs, setBg }: {
  folder: string;
  bg: BackgroundSettings;
  thumbs: BackgroundThumb[] | null;
  setBg: (patch: Partial<BackgroundSettings>) => void;
}): JSX.Element
```

`BackgroundPanel`'s whole Wallpapers tab: one `CategorySection` per `wallpaperGroups` entry, then "Your file" (a `BackgroundAssetCard`) as the last one. It lives here rather than in `BackgroundPanel.tsx` so that file stays a flat read of the panel's flow.

**Which section opens.** `defaultOpenIndex` over `[...each group holds the mesh id, the background IS an imported asset]`, so opening the panel lands on the group the current wallpaper is in - or on "Your file" when the background is the user's own image or video, or on the first group when the background is a solid or a gradient. A closed section that holds the selection names it in its header: the wallpaper's name, or the imported file's name through `assetFileName`.

**Why "Your file" is a section like any other.** Importing is one more way to choose a background, not a separate mode. Picking a wallpaper leaves `asset` alone, so coming back to that section restores the imported file without a re-import. Its count is 1 when a file is imported and 0 when none is, which is what a closed header has to say about it.

### Used by

- `src/editor/panels/BackgroundPanel.tsx` - rendered for `tab === "wallpapers"`.
- `src/editor/panels/GradientTab.tsx` uses `TileGrid` and `thumbGroups` for its own preset sections.
