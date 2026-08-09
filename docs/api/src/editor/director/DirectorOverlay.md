# src/editor/director/DirectorOverlay.tsx

Everything the AI director's choreographed reveal renders at the editor root, bundled into one component so `Editor.tsx` (already at its line budget) only has to mount a single element for the whole feature.

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
- `planning: boolean` / `model?: string` - `Editor`'s `director.planning` and `doc.settings.ai_model || undefined` (Task 40); forwarded straight to `DirectorScrim` for its planning-phase "Asking `<model>`…" pill copy.
- `pointerRef: RefObject<DirectorPointerHandle | null>` - `Editor`'s `director.pointerRef`, forwarded straight to `DirectorPointer`'s `ref`.
- `progress` / `onCancel` - forwarded straight to `DirectorScrim`.

### Behavior

Renders `<AnimatePresence>{running && <DirectorPointer ref={pointerRef} />}</AnimatePresence>` (the fake pointer, conditionally mounted so `AnimatePresence` fades it in/out with the run) followed by `<DirectorScrim running={running} planning={planning} model={model} progress={progress} onCancel={onCancel} />` (the cancel scrim + Stop pill, which owns its own internal mount gating on `running`).

### Used by

`Editor` (`src/editor/Editor.tsx`) - rendered as a sibling of `.e-body`, right before `Transport`.
