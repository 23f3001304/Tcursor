# src/editor/panels/CursorPackGrid.tsx

The Cursor panel's pack picker, split out of `CursorPanel.tsx` to keep that file under its line budget. One collapsible section per pack STYLE, each holding a wrapping grid of tiles three to a row. Each tile rests on the pack's ARROW sprite and, while hovered, walks its nine states so the user sees what they are choosing before choosing it - including the busy state animating exactly as the export will render it.

**Where the sprites come from.** `CursorPackInfo.dir` is the pack's folder on disk and `CursorPackInfo.files` names the file for each kind, so a tile loads `<dir>/<files[kind]>` through the asset protocol (`fileSrc`). The backend never base64s them: 15 packs x 9 PNGs is about a megabyte per reply, and each would have to go through `decode_sprite`, which shells out to ffmpeg - opening this panel would launch 135 subprocesses. This way the browser fetches only the files a tile actually shows.

**Why `files` and not `<kind>.png`.** Two rules the frontend would otherwise have to duplicate live in that map already: the embedded pack's folder spells its arrow `pointer.png`, and any pack with a still busy state renders its ARROW for Busy (`pack::busy_as_arrow`). Resolving both server-side is what keeps a tile showing exactly what picking the pack draws, for every pack kind, with no client-side special cases. The embedded "Default" pack therefore cycles like any other rather than showing a bare name.

**Styling (panel pass, 2026-09-13).** The tiles are the shared idiom: `PackTile` (`PackTile.md`), in `panels/panels.css`. The old `.e-pack-grid`/`.e-pack-card` rules are gone, and with them a fixed five-column grid that left dead space on the right.

**Layout (arrangements pass, 2026-09-14): a collapsible section per style, holding a wrapping grid.** The usability pass had made this a sideways strip per group, which bought the height back but hid most of every group behind a flick and left the pack's NAME in a tooltip that the strip then clipped. The owner rejected all of that: no horizontal movement in a panel, a section per category, and the name under the picture.

What it costs now: six closed section headers are 28px each, and the one section that opens (the style the current pack is in, `defaultOpenIndex`) is one or two rows of 78px tiles. That is less than the 84px strip plus its group label, and nothing is hidden behind a gesture.

**Why the sections are STYLES and not "built in vs imported".** The owner's read: provenance says nothing about what a pack looks like, which is the only thing anyone is choosing between here. The grouping key is `CursorPackInfo.category`, which each pack states in its own `pack.json` - so this file holds no list of pack ids and a new pack joins a section by shipping a folder. `packCategories.md` has the ordering rule; `src-tauri/assets/cursorpacks/README.md` documents the field. "Imported" is one section among the styles, which is also why the `(built in)` tooltip suffix is gone: the heading above a tile already says it.

**What the owner's read of this grid changed.** Every pack's PNG is 128x128 with the drawing padded differently inside it (content widths run 70 to 105px) and with the pack's own colours (Cartoon orange, Cat cream, Ink black). Tiles therefore looked mismatched in both size and value. The fix is entirely in `PackTile`: one constant plane per tile, a constant mid-grey plate behind each glyph so a dark pack is legible without being recoloured, and each sprite scaled by its own alpha content box so every arrow is the same visual size. The glyph itself is never tinted, filtered or shadowed - what a tile shows is what the pack draws.

## KINDS

```ts
const KINDS: string[]
```

The nine cursor states, in the order a hovered tile walks through them. The same wire names the backend uses for PNG filenames (`CursorType`'s serde names), so a tile's sprite path is just `<dir>/<kind>.png` - the two can never drift.

## KIND_MS

```ts
const KIND_MS = 400
```

How long the hover preview holds each state. `LAP_MS` is one full pass through all nine.

## spriteSrc

```ts
function spriteSrc(pack: CursorPackInfo, kind: string, frame: number): string
```

One pack's sprite file, through the asset protocol. The backend already resolved every name into `pack.files`, so this only special-cases explicit busy frames, which are numbered rather than named.

Returns `""` when the pack has no folder, or does not ship that kind - the caller then leaves the previous state up rather than blanking the tile, because the renderer falls back to the embedded sprite there and the grid cannot display `include_bytes!`.

## useHoverCycle

```ts
function useHoverCycle(pack: CursorPackInfo, hovered: boolean,
                       img: React.RefObject<HTMLImageElement | null>): void
```

Walk `img` through the nine states while `hovered`, animating the busy one with the SAME `busyPose` the export and the canvas preview run - so a tile shows exactly what picking that pack will render.

### Why it writes to the DOM instead of setting state

The loop runs at rAF cadence. Holding the current state in `useState` would re-render the panel 60 times a second for as long as a pointer rests on a tile; writing `src`/`style.transform` straight onto the element costs nothing above the browser's own compositing. `src` is assigned only when the file actually CHANGES (tracked in `shown`), or the browser re-decodes the same PNG every frame.

### Why one loop, not one per tile

The effect keys on `hovered`, so exactly one tile is ever animating and an idle grid costs nothing. Both the cleanup and the not-hovered branch call `rest()`, which puts the tile back on its arrow - otherwise a tile would be left frozen mid-cycle on whatever state the pointer left it at.

*Cycle timing:* `lap = elapsed % LAP_MS` first, then the state index from that - so the busy state's own `t` (`lap - i * KIND_MS`) restarts at 0 on every pass instead of drifting with total hover time.

## CursorPack

```tsx
function CursorPack({ pack, selected, onPick }: {
  pack: CursorPackInfo; selected: boolean; onPick: () => void;
}): JSX.Element
```

One tile: a shared `PackTile` holding a `GlyphPlate`, plus the hover cycle wired to the plate's `<img>`. Renamed from `PackTile` in the panel pass, when the tile shell itself became the shared component of that name.

`title` is the pack's plain name. It carried a `(built in)` suffix until the arrangements pass; the section heading now carries that, and the name itself is captioned under the tile rather than living only in a tooltip.

The cycle writes only `src` and `transform`; the element's size and offset are the plate's fitted ones, so every state of every pack stays the same visual size. Hover is a CSS lightness step on the tile (no lift - a grid where every tile jumps reads as jelly), and the press spring is `PackTile`'s. There are no CSS keyframes anywhere in this file.

## CursorPackGrid

```tsx
export function CursorPackGrid({ packs, selected, onPick }: {
  packs: CursorPackInfo[]; selected: string; onPick: (id: string) => void;
}): JSX.Element
```

The sections themselves: `packCategories(packs)` (`packCategories.md`) splits the list by `category` into the curated order, and each entry becomes a `CategorySection` (`CategorySection.md`) wrapping one `.e-tile-grid`, all inside one `.e-secstack` so the headers sit at 2px rather than the 10px of the `.e-grp` this renders into. Empty categories never appear, so a user with no imports sees no "Imported" heading.

`defaultOpenIndex` opens exactly the section holding `selected`, and that section's header also carries the chosen pack's name while closed - so the picker's whole state is readable without opening anything. Nothing is written to storage until the user toggles a section themselves.

Every tile stays a plain button in the tab order rather than a listbox option (`PackTile.md` says why); a closed section's tiles are unmounted, so Tab crosses the picker in a handful of stops.

`selected` is `CursorSettings.pack`; `onPick` writes it. The backend guarantees one row per id (`pack::list_packs` dedupes), which is also what makes `p.id` a safe React key.

### Used by

- `src/editor/panels/CursorPackField.tsx` - renders it once the pack list resolves (the fetch and the import moved there in the panel pass; `CursorPanel` no longer touches either)
