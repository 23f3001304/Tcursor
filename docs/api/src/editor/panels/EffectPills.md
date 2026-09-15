# src/editor/panels/EffectPills.tsx

The Effects panel's insert palette: four cards that each add one element to the timeline, by click or by drag. Layout Segment, Zoom Region, Spotlight Highlight, Camera Move.

## EffectPills

```tsx
export function EffectPills({ onAddZoom, onAddSpotlight, onAddLayout, onAddCameraMove }: {
  onAddZoom: () => void; onAddSpotlight: () => void; onAddLayout: () => void; onAddCameraMove: () => void;
}): JSX.Element
```

Each card is a plain `draggable` div carrying `text/plain` = the element type, plus an `onClick` for the add-at-playhead path. The four callbacks are passed straight through from `EffectsPanel`; this file does not know how an element gets added, only that a pill was used.

### Why plain divs and not `motion.div`

Motion's gesture layer takes pointer capture, which stops `dragstart` from firing at all (commit `b23b73d` - `whileHover` alone was enough to break it). So the hover lift here is a plain CSS `translateY` transition rather than the Motion spring the rest of the panels use. It is the one documented exception to the one-motion-language rule, and the reason is mechanical rather than aesthetic.

### Look

Each card carries its lane's own accent in its glyph and its 12% tint (`--pill-accent`, the same custom property the timeline's own pills set), because these four ARE the timeline's four pill kinds and should read as the same object family. The panel pass removed their borders, and the 2026-09-15 pass removed the 3px accent bar they and every timeline pill used to carry on the left edge: the owner read it as a cheap card rail, so the colour now lives in the tint and the glyph alone. The rules moved from `editor.css` to `panels/panels.css`, as the combined `.e-zblk.e-libpill` selectors that beat the timeline's own shared rule regardless of source order.

Since the panel pass the group's heading is `.e-sechead` ("Insert timeline elements") rather than a control label, because it heads a group rather than naming one control.

### Used by

- `src/editor/panels/EffectsPanel.tsx` - the first group in the panel: what you can add comes before what is always on.
