# src/editor/EffectInspector.tsx

Left-panel inspector for the selected effect region (spotlight), shown in place of the tab content while an effect region is selected. v1 edits start/end + delete; the spotlight's look comes from Settings (per-region params are a later addition).

## EffectInspector

```tsx
export function EffectInspector({ effect, dur, onApply, onClose }: {
  effect: EffectRegion; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `effect: EffectRegion` - the selected region.
- `dur: number` - clip duration (bounds the end input).
- `onApply` - the start/end number inputs apply an `update_effect` op; the delete button applies `remove_effect` then closes.
- `onClose` - deselect (clears the selection in `Editor`).
