# src/editor/panels/WallpaperGrid.tsx

The background picker's tile rows, shared by `BackgroundPanel`'s Wallpapers tab and `GradientTab`'s preset row. Every tile is a 96x54 PNG rendered in Rust by the export's OWN background code (`background_thumbs`), so what the row shows is what the render produces - not a CSS lookalike that drifts from it as the two are edited apart. That is also why the old `GRADIENT_PRESETS` CSS strings in `backgroundPresets.ts` were removed rather than reused here.

**The file is still called `WallpaperGrid.tsx` and its component is `WallpaperRow`.** The usability pass turned each group from a wrapping grid into one horizontally scrolling strip (`TileRow.md` carries the arithmetic); the file name stayed because the data helpers below - which are what most callers actually import - did not change at all.

## Tile

```ts
export interface Tile { id: string; name: string; png: string; css: string }
```

One tile's display data.

- `id` - what the caller gets back from `onSelect` (and, for wallpapers, exactly what `BackgroundSettings.mesh` stores).
- `name` - the tile's `title`, and the caption that rides in over the artwork on hover or keyboard focus. *Why a caption at all:* a strip tile is 64x36, and several wallpapers are deliberately dark and calm - without a name they are hard to tell apart at that size.
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

## wallpaperGroups

```ts
export function wallpaperGroups(thumbs: BackgroundThumb[]): { name: string; tiles: Tile[] }[]
```

The wallpaper tiles split into the picker's groups - one ROW each since the usability pass. Groups are cut where the backend's `group` field changes, so the ORDER is whatever Rust emitted (`Ribbons`, `Folds`, `Gradients`, `Metal`, `Scenic`, each alphabetical) - a new group ships by dropping files into `assets/`, with no change here or in the panel.

`CLASSIC` is unshifted onto the FIRST group, so the legacy background stays one click away without needing a row of its own. With no thumbnails at all - no ffmpeg, or the fetch rejected - the function returns a single `Wallpapers` group holding just Classic, because Classic renders through a path that never needed the thumbnail list and the tab must not come up empty.

### Behaviors

- `splits the wallpapers into the backend's own sections, in its order`.
- `puts Classic first in the first section, never in a later one`.
- `still offers Classic when no thumbnails came back at all`.
- `renders one listbox row per group, named by the group, with Classic leading the first` (`BackgroundPanel.test.tsx`).

## WallpaperRow

```tsx
export function WallpaperRow({ label, tiles, selectedId, onSelect }: {
  label: string;
  tiles: Tile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element
```

One group as one `TileRow` (`TileRow.md`): a horizontally scrolling strip of 64x36 tiles, with the group's name as the row's own 11px label rather than a section heading above a grid.

**What changed, and why.** The panel pass had already made the tile BE the artwork - no padding, no border, a 2px inset accent ring for selection, the name as a caption over the bottom on hover. The usability pass kept all of that and changed only the layout: `repeat(auto-fill, minmax(88px, 1fr))` three-up meant the 52-tile library cost about 1000px of a 620px panel, and the owner's verdict on it was that the Background panel "needs a lot of scrolling". Five strips cost 295px. Keyboard, scroll-into-view and the listbox semantics all come from `TileRow` and are documented there.

- `label` - the group name, which doubles as the row's `aria-label`.
- `selectedId` - `null` means nothing in THIS row is active, which is the normal state for every wallpaper row while the background is a gradient or a solid, and for the gradient row once the user has edited a preset's stops. Compared against `Tile.id`, so the Classic tile is selected exactly when `mesh` is empty.
- `onSelect` - handed the tile's id; the caller decides what that means (a `mesh` id, or a preset to expand into gradient stops).

### Used by

- `src/editor/panels/BackgroundPanel.tsx` - one row per wallpaper group (`CLASSIC` prepended to the first).
- `src/editor/panels/GradientTab.tsx` - the twelve gradient presets, as one row labelled Presets.
