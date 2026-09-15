# src/editor/panels/captions/CaptionColorFields.tsx

The colour half of the Captions panel's Look block: text colour, the spoken-word highlight, and the background pill with its own colour and opacity. Split out of `CaptionStyleControls.tsx` when the owner's 2026-09-15 round added colour, fine sizing and animation to what had been four switches - the two halves together are past the file budget, and colour is the half that reads as one subject.

## CaptionColorFields

```tsx
export function CaptionColorFields({ style, accent, onChange }: {
  style: CaptionStyle;
  accent: [number, number, number];
  onChange: (s: CaptionStyle) => void;
}): JSX.Element
```

Three fields, each a `Swatches` row plus a `ColorInput` for anything the row does not offer, laid out by `.e-caprow-colors` (`panels.css`) so the custom well sits at the end of the swatches rather than under them.

Like `CaptionStyleControls`, every change hands the WHOLE `CaptionStyle` back through `onChange`, so one click is one undo step and the preview follows on the same tick.

### The Accent swatch

The highlight row's FIRST swatch writes `highlight_color: null`, which is not a colour - it is the instruction "use the interface accent", and it is what every project has meant since captions existed. It is drawn in the project's real accent (`doc.settings.ui.accent`, which is why the component takes `accent` as a prop rather than reading a CSS token): a chip showing some other red while the accent is blue would be a lie about what picking it does.

The `ColorInput` beside it falls back to `accent` for its own swatch when `highlight_color` is `null`, so the well shows the colour currently in effect rather than black.

Swatch selection compares through `rgb()` (`panels/effectSwatches.ts`) rather than by array identity, and the `null` case compares by identity - a colour equals a colour, and Accent equals only Accent.

### What hides when

The highlight fields are mounted only while `highlight` is on, and the pill's colour and opacity only while `pill` is on. A control that cannot change anything is worse than an absent one: it invites a click and answers with nothing.

### Used by

- `src/editor/panels/captions/CaptionStyleControls.tsx`
