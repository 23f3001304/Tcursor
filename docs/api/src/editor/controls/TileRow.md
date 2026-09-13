# src/editor/controls/TileRow.tsx

One horizontally scrolling row of picture tiles. Added by the **usability pass** (2026-09-13) as the replacement for the preset GRID, and the single biggest reason the Background panel now fits its slot.

**The arithmetic it exists for.** The wallpaper library is 52 bundled images in five groups. Three-up in a 288px content column that is 18 wrapped rows at 51px plus five group headings: roughly 1000px, in a panel slot that is about 620px tall on a 720px window. The same 52 as five strips is 295px, and every tile is still one flick (or one arrow key) away. The scrolling moved from the panel, which has nowhere to go, into the row, which does.

**Where the scrolling is allowed to happen.** The row scrolls; the panel never does because of it. `overscroll-behavior-x: contain` stops a fling at the end of a strip from turning into a page scroll, and the scroll-into-view below asks for `nearest` on both axes so walking a row never drags the panel up or down under the user.

**The scrollbar lane is reserved, not conjured.** `::-webkit-scrollbar` is 4px tall at all times with a transparent thumb; hover only changes the thumb's colour. Growing the scrollbar on hover would shove every tile up 4px, which is the exact twitch this pass set out to remove.

## RowTile

```ts
export interface RowTile { id: string; name: string; content: ReactNode }
```

One tile.

- `id` - what `onSelect` hands back, and what `selectedId` is compared against.
- `name` - the tile's `title` and the caption that rides in over its face on hover or keyboard focus. Never hover-only information, because `title` carries it too.
- `content` - whatever paints the 64x36 face. The wallpaper rows pass a `.e-tile-img` span; nothing about the row assumes it is an image.

## rowNextIndex

```ts
export function rowNextIndex(key: string, index: number, length: number): number | null
```

The tile index an arrow, `Home` or `End` press should move to, or `null` for any other key.

It **clamps** at both ends, which is the one place it differs from `segmentedNextIndex` (`Segmented.md`), and the difference is deliberate: a segmented control is two to four segments wide, so wrapping reaches any of them faster than reversing; a tile row can hold thirteen, and running off the end of a strip should stop where the eye is rather than teleport to the far side of it. From "this row holds no selection" (`index < 0`) it steps from the first tile.

### Behaviors

- `steps forward and back along the row`.
- `clamps at both ends rather than wrapping`.
- `treats the vertical arrows the same as the horizontal ones`.
- `jumps to the ends on Home/End and ignores everything else`.
- `starts from the first tile when this row holds no selection, and refuses an empty row`.

## TileRow

```tsx
export function TileRow({ label, ariaLabel, tiles, selectedId, onSelect }: {
  label?: string;
  ariaLabel: string;
  tiles: RowTile[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}): JSX.Element
```

### Props

- `label` - the row's own 11px dim label, which is what a group's section heading became. The Background panel passes the backend's group name straight through ("Ribbons", "Folds", "Metal"), so a new group ships by dropping files into `assets/` and needs no change here or in the panel.
- `selectedId` - `null` means nothing in THIS row is active, which is the normal state for every wallpaper row while the background is a gradient or a solid colour.

### Accessibility

The wrapper is `role="listbox"` with `aria-label`; each tile is a `role="option"` button with `aria-selected`. **Roving tabindex**: the selected tile is the row's one tab stop, and the first tile stands in for a row that holds no selection (without that, a row nobody has picked from yet would be unreachable by keyboard). So Tab crosses a five-row wallpaper library in five stops rather than fifty-three, and the arrows walk within a row.

**Selection follows focus.** An arrow press both moves the selection and applies it, which is what makes a row auditionable: hold an arrow down and the stage repaints through a whole group. That is the listbox convention for a single-select list whose choices are cheap to apply, and every choice here is one settings write that undo already covers.

### Scroll into view

An effect keyed on `selectedId` finds the `aria-selected` tile and calls `scrollIntoView({ block: "nearest", inline: "nearest" })`. It fires on mount, so reopening a panel lands on what is actually selected however deep in the strip it sits, and after each arrow press, which is what keeps keyboard walking visible. The optional call (`?.`) is for jsdom, which implements no `scrollIntoView` at all.

### Motion

`whileTap` scale 0.96 on the app's press spring (500/30), dropped under `useReducedMotion` - the same press `PackTile` has, because a strip tile is that tile at a smaller size. Hover, focus and the caption's fade are CSS transitions at 0.14s; nothing here animates a layout property.

### Styling

`.e-tilerow-wrap` / `.e-tilerow-label` / `.e-tilerow` / `.e-rowtile` / `.e-rowtile-cap` in `controls/controls.css`. The tile is the same card language as `.e-tile` (`panels.css`): a raised plane, one lightness step on hover, a 2px inset accent ring when chosen, and never a tint or dim over the tile's own picture. `.e-tilerow`'s scroller rules are shared with `.e-tile-strip`, the cursor packs' bigger-tiled version of the same idiom.

### Used by

- `src/editor/panels/WallpaperGrid.tsx` (`WallpaperRow`) - the five wallpaper groups and the gradient presets.
- `src/editor/panels/BackgroundAssetCard.tsx` renders the same classes by hand for the Custom row, which is a mixed row (an import button plus a card) rather than a listbox of choices.
