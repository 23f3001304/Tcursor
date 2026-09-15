# src/editor/panels/captions/CaptionStyleControls.tsx

How captions look, for the whole project: where they sit, how big they are, what colour they are, and how they arrive. The owner's 2026-09-15 review of a real 87-caption take asked for "colour, their size and the way they come, and different effects for caption animation", and this is where all three live. Everything the renderer still pins by itself (the font, the margins, the 42-character wrap budget) is identical in `export/fx/caption/captionlayout.rs` and `stage/fx/captionPreview.ts` so the preview and the export cannot disagree.

The colour fields are a sibling file, `CaptionColorFields.tsx`; the size rungs, slider ranges, animation menu and palettes are data in `captionLook.ts`.

## DEFAULT_CAPTION_STYLE

```ts
export const DEFAULT_CAPTION_STYLE: CaptionStyle
```

Mirrors Rust `settings::captions::CaptionStyle::default()` field for field, the new fields included: `font_pct: 0` (use the `size` rung), white text, `highlight_color: null` (the interface accent), a black pill at 62% (the old `SCRIM`), and a 120 ms fade (the old `FADE_MS`). Those defaults reproduce the previous look exactly, which is what made the fields safe to add to projects that never had them.

It is what the panel header's Reset puts the LOOK fields back to. `model` and `language` are in the same struct but are not a look, so Reset keeps them: sending a user back to a model download because they wanted the pill off would be a surprise, not a reset.

## CaptionStyleControls

```tsx
export function CaptionStyleControls({ style, accent, onChange }: {
  style: CaptionStyle;
  accent: [number, number, number];
  onChange: (s: CaptionStyle) => void;
}): JSX.Element
```

A master switch, then Position, Size, Line height, the colour fields, Animation and Duration - all hidden while the master switch is off, because there is nothing to position when nothing is drawn. `accent` is the project's interface accent, passed straight through to `CaptionColorFields` for its Accent swatch.

Each change hands the WHOLE `CaptionStyle` back, and `CaptionsPanel` writes it as `saveDocSettings({ ...doc.settings, captions })` - the shape every other panel uses, and one undo step per change. The preview follows on the same tick with no second path to keep in step: `ClassicShell` already passes `doc.settings.captions` into `useSyncRefs`, which is the `capStyleRef` the composite loop reads on every frame.

### Size: three rungs and a slider, one number

Size is a `Segmented` of Small / Medium / Large AND a Line height slider (2.0% to 6.0% of frame height, in tenths), and they are two views of the single field `font_pct` rather than two settings that can disagree:

- Picking a rung writes `font_pct` to that rung's value AND `size` to the rung, so the two halves of the model stay coherent whichever one the renderer reads.
- Dragging the slider writes `font_pct` alone.
- A rung lights only while the line height is exactly on it (`captionRung`), so dragging between two rungs leaves all three dark - the honest answer, and the one that stops a lit rung from claiming a size the caption does not have.
- A project that never touched the slider (`font_pct: 0`) reads its rung's value, so the slider opens on 3.8 and Medium is lit rather than the thumb sitting at zero.

The readout says "3.8% of height" - a bare percentage next to a caption reads as an opacity.

### Animation

A `Picker`, not a `Segmented`: five options with a sentence each do not fit five abreast in a 320px panel, and the sentence is the point. Every option carries a one-line `title` (see `captionLook.md`) saying what the viewer sees. The Duration slider (0 to 600 ms, 20 ms steps) is mounted only when the animation is not None - there is no duration for a cut.

### Used by

- `src/editor/panels/captions/CaptionsPanel.tsx`
- `src/editor/shell/shellFixture.tsx` - `DEFAULT_CAPTION_STYLE` seeds the test document's settings.
