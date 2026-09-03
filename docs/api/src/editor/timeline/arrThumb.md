# src/editor/timeline/arrThumb.ts

The pure geometry behind the T34 L4 layout thumbnail: scales a segment's RESOLVED panels (`resolvedPanelsFor`, T34 L2's per-segment channel) into a tiny schematic box. `LayoutThumb.tsx` draws what this computes; this file has no rendering knowledge at all.

## THUMB_W

```ts
export const THUMB_W = 24, THUMB_H = 14;
```

The schematic's own coordinate box, in unit-less "thumb px". `LayoutThumb.tsx`'s `<svg viewBox="0 0 24 14">` uses these SAME numbers, so drawing the thumbnail bigger (`LayoutInspector`'s 48x28 header) is a pure SVG/CSS scale-up of identical geometry - `arrThumb` is never called twice with different box sizes for the same segment in practice, though the `w`/`h` params below exist for exactly that if a caller ever needs a genuinely different-shaped box.

## ThumbBox

```ts
export interface ThumbBox { x: number; y: number; w: number; h: number }
```

One panel's box inside the schematic, in the same units as `w`/`h` below.

## ArrThumb

```ts
export interface ArrThumb { screen: ThumbBox | null; cam: ThumbBox | null }
```

A segment's schematic: each field is a `ThumbBox`, or `null` when that panel isn't shown.

## arrThumb

```ts
export function arrThumb(panels: ResolvedPanels | null, w = THUMB_W, h = THUMB_H): ArrThumb
```

Scales `panels.screen`/`panels.cam` into the `w`x`h` box. `PanelRectDto.rect` is already a `0..1` fraction of the output frame (`preview_layouts.rs`'s `panel_dto`), so "normalizing into the thumb box" is literally `rect[i] * w|h` - zero pose math, the same rule `layoutTrack.ts` follows (this only re-scales an already-resolved rect, it never derives one).

A panel resolved at or below `VISIBLE_ALPHA` (`../stage/arrange/arrangeMath.ts` - the same threshold `ArrangeOverlay`/`LayoutInspector` use to decide whether a panel counts as hidden) draws as `null`, not a zero-size box - a hidden `camera_only` screen or a switched-off panel shows NOTHING in the schematic, matching what the stage actually renders.

`panels === null` - presets haven't loaded yet (`useEditorData`'s first `previewLayouts` fetch hasn't landed), or there's no segment to show - returns `{ screen: null, cam: null }` rather than guessing a placeholder box; `LayoutThumb.tsx` falls back to a plain icon in that case.

### Used by

`LayoutThumb.tsx` (`./LayoutThumb.tsx`) - the only caller; everything downstream (the layout pill, `LayoutInspector`'s header) goes through that component, never this function directly.

Unit-tested (`arrThumb.test.ts`): a full-screen preset, a small-cam preset at both the pill's 24x14 and the inspector's 48x28 sizes, a hidden screen (`camera_only`-style) and a hidden cam (`screen_only`-style) symmetrically, a custom arrangement's arbitrary rect (numbers matching none of the five presets, so a pass proves this reads the segment's OWN resolved rect), the `panels === null` fallback, and the exact `VISIBLE_ALPHA` boundary.
