# src/editor/panels/EffectPills.tsx

The Effects panel's insert palette: eleven cards that each add one element to the timeline, by click or by drag. Layout Segment, Zoom Region, Spotlight Highlight, the three masks (Blur, Pixelate, Highlight), Camera Move and the four text kinds (Title, Lower Third, Big Stat, Callout).

## PillActions

```ts
export interface PillActions {
  onAddZoom: () => void; onAddSpotlight: () => void; onAddMask: (kind: MaskKind) => void;
  onAddLayout: () => void; onAddCameraMove: () => void; onAddText: (kind: TextKind) => void;
}
```

Everything a pill can do. `EffectPills` takes this object as its whole props, so a new row costs a table entry and, only when it is a genuinely new action, one field here. The three masks share ONE callback discriminated by `MaskKind` rather than taking three, because they differ only in the kind they pass to the same op; the four text kinds share `onAddText` for the same reason.

`onAddMask` and `onAddText` are the two callbacks here that take an argument, because a mask or a text pill has to say WHICH kind to seed. The other three elements have exactly one shape each.

## PILLS

```ts
export const PILLS: Pill[]
```

The rows, in the order they are shown: `type` (the `text/plain` drag payload AND the key), `cls` (the lane's pill class), `Icon`, `name`, `hint`, and `run(actions)`.

*Why a table rather than eleven JSX blocks.* At four rows the repeated markup was tolerable; at eleven it is the same eleven lines copied eleven times. A table makes a row one entry and keeps the file flat and under its budget, and it is what lets `effectPills.test.tsx` assert the rows as DATA - name, drag type, accent class and the callback each `run` reaches.

**The four text rows are generated, not written out.** They are spread in from `TEXT_PILLS` (`./textStyles.ts`) paired with `TEXT_ICONS` here, because they differ only in kind, name, hint and glyph; both the click (`onAddText(kind)`) and the drag payload (`text:<kind>`) are built from the row, so `Timeline`'s `onDrop` needs one `startsWith("text:")` arm rather than four.

**The `type` is load-bearing in two places at once:** it is the drag payload `Timeline`'s `onDrop` switches on, so a row's `type` and the arm that receives it must agree. The test pins that every `type` is distinct.

## EffectPills

```tsx
export function EffectPills(a: PillActions): JSX.Element
```

Renders `PILLS`. Each card is a plain `draggable` div carrying `text/plain` = the row's `type`, plus an `onClick` that calls the row's `run`. The callbacks are passed straight through from `EffectsPanel`; this file does not know how an element gets added, only that a pill was used.

### Why plain divs and not `motion.div`

Motion's gesture layer takes pointer capture, which stops `dragstart` from firing at all (commit `b23b73d` - `whileHover` alone was enough to break it). So the hover lift here is a plain CSS `translateY` transition rather than the Motion spring the rest of the panels use. It is the one documented exception to the one-motion-language rule, and the reason is mechanical rather than aesthetic.

### Look

Each card carries its lane's own accent in its glyph and its 12% tint (`--pill-accent`, the same custom property the timeline's own pills set), because these ARE the timeline's pill kinds and should read as the same object family. The three masks all carry `e-fxblk`, the spotlight's own class, because they share the FX lane and its `--e-fx` accent; their icons are what tell them apart, exactly as on the lane itself. The panel pass removed their borders, and the 2026-09-15 pass removed the 3px accent bar they and every timeline pill used to carry on the left edge: the owner read it as a cheap card rail, so the colour now lives in the tint and the glyph alone. The rules moved from `editor.css` to `panels/panels.css`. They are keyed on `.e-libgrid > .e-libpill`, two classes against the one class of the timeline's shared lane rule (`position: absolute; top: 0; height: var(--e-row-h)`), so they win regardless of source order and whatever lane class a pill carries. They used to name each lane class (`.e-zblk.e-libpill`, `.e-fxblk.e-libpill` and so on); the four Text pills carry `e-textblk`, which that list never gained, so they kept the lane's absolute positioning and stacked in the window's top-left corner over the title. `effectPills.test.tsx` reads the stylesheet and checks that the rule matches every rendered pill, so a new pill kind cannot miss it.

Since the panel pass the group's heading is `.e-sechead` ("Insert timeline elements") rather than a control label, because it heads a group rather than naming one control.

### Used by

- `src/editor/panels/EffectsPanel.tsx` - the first group in the panel: what you can add comes before what is always on.
