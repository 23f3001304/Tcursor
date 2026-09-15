# src/editor/motion/PresetRow.tsx

The named starting points above `MotionGraph`. Presets are named by FEEL - Snappy, Soft, Cinematic, Mechanical, Bouncy - never by the curve underneath, which is the whole point of the motion editor: a curve is something you draw, a preset is something you recognise (`docs/superpowers/specs/2026-09-15-motion-editor-design.md` 3).

**The list is a prop.** This file imports no preset table. The inspectors, the timeline's curve popover and Settings' Motion section all show the same row, and each hands over whatever it offers; a row that reached for `presets.ts` itself would make every one of those callers depend on that module's shape, and would make the row impossible to test with two made-up entries.

It is a plain `Segmented`, the project's own 2-to-6 option control, so the row picks up its keyboard model (arrows wrap, Home and End jump) and its gliding highlight for free - the one place Motion is allowed anywhere near this graph.

## CUSTOM

```ts
export const CUSTOM = "custom";
```

The id shown when the curve matches no preset, the same miss value `presetOf` returns. Named here so a caller can compare against a constant rather than spell the string again at each site.

## PresetRow

```tsx
export function PresetRow(props: {
  presets: { id: string; name: string; feel?: string }[];
  value: string; onPick: (id: string) => void;
}): JSX.Element
```

`value` is a preset id or `CUSTOM`. A preset's `feel` becomes its tooltip, which is where the one-line description belongs: on the row it would be five captions competing with the graph underneath.

**Custom is a sixth segment that only exists while it is the answer.** Offering it permanently would invite a press that means nothing - there is no "custom curve" to switch TO - and a row that silently lit none of its segments read as a broken control. The way to reach Custom is to drag the graph, which is exactly the story the section tells.

**Sizing (panel pass, 2026-09-15).** Six words in the inspector's ~300px is more than `Segmented`'s default fit: at its normal 6px side padding "Cinematic" and "Mechanical" ellipsised. `motion.css` carries one rule keyed on the row's aria-label (`.e-segmented[aria-label="Motion preset"]`) that drops a segment's side padding to 2px and the label one size step (`calc(var(--e-text-hint-size) - 1px)`, tightened by 0.01em), which seats every whole word at the 320px panel and in the 312px camera popover. Keyed on the label rather than a class so this file stays a plain `Segmented` with nothing to pass through.
