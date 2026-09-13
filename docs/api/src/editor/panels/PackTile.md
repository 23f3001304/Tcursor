# src/editor/panels/PackTile.tsx

The panels' ONE card level. Added by the panel pass, which found three near-identical bordered-card rules in `editor.css` (`.e-pack-card`, `.e-wall-tile`, `.e-bgasset`) that had drifted apart in radius, padding and selected treatment. All three became this component and one `.e-tile` rule.

**Scope since the usability pass (2026-09-13): the cursor packs.** The wallpapers and gradients moved to `controls/TileRow.tsx`'s 64x36 strip tile, which is this tile's look at a smaller size and with listbox semantics (`TileRow.md`). The `art` and `caption` props that existed only for them are gone with them. A pack tile is still a 66px column - a glyph plate with the pack's name under it - because a pack name is not optional the way a wallpaper's is.

**The look, and why.** A raised plane with no border; hover is one further lightness step and nothing else; selected is a 2px inset accent ring. Three things follow from the benchmark's section (e) list of cheap tells: bordered cards inside bordered panels read as a spreadsheet (tell 2), so the tile has no stroke at all; a grid where every tile lifts on hover reads as jelly, so the hover is a lightness step only (also the owner's ruling on the pack grid); and a selected tile must never be marked by darkening or tinting its own artwork, because the artwork IS the choice.

## GlyphPlate

```tsx
export function GlyphPlate({ src, imgRef }: {
  src: string; imgRef?: RefObject<HTMLImageElement | null>;
}): JSX.Element
```

A cursor sprite on the shared neutral plate.

The plate is a constant mid-grey square (`--e-plate`, about 35% lightness) drawn behind the glyph. It is what makes an ink-black pack legible on a dark tile *without* touching the glyph: the sprite is never tinted, filtered or drop-shadowed, so a tile shows exactly the colour the pack draws (owner ruling, 2026-09-13). The glyph is scaled by its own alpha content box (`glyphFit.md`) into a fixed 20x24 area, so every pack's arrow comes out the same visual size whatever padding its PNG carries.

`imgRef` lets a caller drive the element directly - `CursorPackGrid`'s hover cycle writes `src` and `transform` on it from a rAF loop, and never touches the size or offset this component computed, so every state of every pack stays the same size.

### useGlyphFit (internal)

Measures `src` once on an offscreen canvas: draw, `getImageData`, `alphaBounds`, `fitBox`. Re-runs only when `src` changes - so a grid measures once per pack, and a hover cycle (which swaps `src` on the DOM element, not through React) never re-measures.

Everything that can fail, fails to the same place: no canvas context (jsdom, a browser refusing the read), a zero-sized image, a fully transparent sprite, or a tainted-canvas `SecurityError` all leave `UNFITTED` in place, which draws the sprite centred at a plain 22px - what the grid did before the fit existed.

## PackTile

```tsx
export function PackTile({ selected, title, label, onPick, onHoverStart, onHoverEnd, children }: {
  selected: boolean; title: string; label?: string;
  onPick: () => void; onHoverStart?: () => void; onHoverEnd?: () => void; children: ReactNode;
}): JSX.Element
```

One choosable tile. `children` is the face, which today is always a `GlyphPlate`.

### Props

- `title` - tooltip AND `aria-label`. Not optional: every tile is a picture, so it always needs a name (benchmark tell 5 - icon-only affordances with no label assume icon literacy).
- `label?` - a caption UNDER the face, always visible. The pack strip uses it: pack names differ in ways the sprites do not.

`aria-pressed` mirrors `selected`. The pack tiles stay plain buttons rather than becoming listbox options the way `TileRow`'s do, because each one owns a hover preview (nine states, a rAF loop) that a roving-tabindex row would have to grow callbacks for; every tile being its own tab stop is also the behaviour the pack strip has always had.

### Motion

`whileTap` scale 0.96 on the app's press spring (500/30), dropped under `useReducedMotion`. Hover is CSS (a background-colour step), not Motion - there is no transform in the hover state to animate.

### Used by

- `src/editor/panels/CursorPackGrid.tsx` - one tile per pack, with `GlyphPlate` as the face. The only caller.
