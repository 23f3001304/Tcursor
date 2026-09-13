# src/editor/director/DirectorOverlay.tsx

Everything the AI director's choreographed reveal renders, bundled into one component so `Editor.tsx` (already at its line budget) only has to mount a single element for the whole feature. Nested inside `.e-stagetoast` (Task 11) rather than at the editor root - the fake pointer and the cancel scrim are both `position:fixed` so where this mounts in the DOM doesn't move them, but the pass's sweeping wave and the Stop pill (`DirectorScrim`) are in flow, so they anchor to the stage's own bottom edge instead of the viewport's (ux audit #17).

## DirectorOverlay

```tsx
export function DirectorOverlay({ running, planning, model, pointerRef, progress, onCancel }: {
  running: boolean;
  planning: boolean;
  model?: string;
  pointerRef: RefObject<DirectorPointerHandle | null>;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}): JSX.Element
```

### Props

- `running: boolean` - `Editor`'s `running` state; gates both children.
- `planning: boolean` / `model?: string` - `Editor`'s `director.planning` and `doc.settings.ai_model || undefined` (Task 40); forwarded to `DirectorScrim` for its planning-phase "Asking `<model>`…" pill copy, and `planning` additionally decides whether the wave is indeterminate.
- `pointerRef: RefObject<DirectorPointerHandle | null>` - `Editor`'s `director.pointerRef`, forwarded straight to `DirectorPointer`'s `ref`.
- `progress` / `onCancel` - forwarded to `DirectorScrim`; `progress` also drives the wave's head.

### Behavior

Three pieces, in order:

1. `<AnimatePresence>{running && <DirectorPointer ref={pointerRef} />}</AnimatePresence>` - the fake pointer, conditionally mounted so `AnimatePresence` fades it in and out with the run.
2. The pass's `SweepWave` (`tone="ai"`), inside its own `AnimatePresence` so it fades in and out on the same 0.16s tween as the pill below it. **This is the pass's progress readout**, and it replaced the indeterminate pulsing dot the Stop pill used to carry - panel-design benchmark (c) item 7, and (e) item 6: a generic spinner for AI processing is a cheap tell at the one moment the brand should feel most in control. While `planning` it sweeps on its own clock, because there is no step count yet; the moment the reveal starts it switches to a determinate head at `progress.step / progress.total`. `pct` is `undefined` whenever `planning` is true, `progress` is null, or `progress.total` is 0, so a zero total can never divide.
3. `<DirectorScrim ... />` - the cancel scrim and the now text-only Stop pill, which owns its own internal mount gating on `running`.

### Used by

`Editor` (`src/editor/Editor.tsx`) - rendered inside `.e-stagetoast`, alongside `Stage` and `Toast` (Task 11; previously a sibling of `.e-body`, right before `Transport`).
