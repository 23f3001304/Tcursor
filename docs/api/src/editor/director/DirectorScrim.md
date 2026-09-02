# src/editor/director/DirectorScrim.tsx

The cancel surface for a running AI-director pass.

## DirectorScrim

```tsx
export function DirectorScrim({ running, planning, model, progress, onCancel }: {
  running: boolean;
  planning: boolean;
  model?: string;
  progress: { step: number; total: number } | null;
  onCancel: () => void;
}): JSX.Element
```

### Props

- `running: boolean` - whether an AI-director pass is in flight (`Editor`'s `running` state). Both the scrim and the Stop pill are mounted only while this is true.
- `planning: boolean` - `Editor`'s `director.planning` (Task 40) - `true` only while the `ai_plan` fetch is in flight, `false` once it settles (whether it resolved, rejected, or was cancelled). Switches the Stop pill from its planning-phase copy to the reveal's step-count copy.
- `model?: string` - `doc.settings.ai_model || undefined`, for the planning-phase pill's `"Asking <model>…"` text. Falls back to `"the model"` when unset (auto-pick).
- `progress: { step: number; total: number } | null` - live position, from `Editor`'s `director.progress`. `null` while the plan is still being fetched (before the first step starts) - `planning` is what actually distinguishes that window now, `progress` alone is ambiguous (also `null` for one tick before `reveal`'s first `setProgress`).
- `onCancel: () => void` - `Editor`'s `director.requestCancel` - sets the `cancelRef` `useDirector.ts`'s `planOrCancel` (during planning) and `reveal` loop (during the reveal) both check.

### Behavior

**Scrim.** A transparent, full-viewport `.e-director-scrim` (`pointer-events: all`, NO visual dim - deliberately invisible, it only exists to catch input) that calls `onCancel` on ANY `pointerdown`. Mounted/unmounted directly on `running` (no exit animation - it should disappear the instant the run ends, not fade).

**Escape.** A `keydown` listener, attached only while `running` (removed the moment it flips false), calls `onCancel` on `Escape`.

**Stop pill.** A `motion.div.e-director-stop` (Task 11: now positioned `position:absolute` inside `Editor.tsx`'s `.e-stagetoast` - the same stage-bottom container `Toast` anchors in, `bottom: 60px` vs. `Toast`'s own `22px` so the two stack rather than overlap if an undo/redo is queued during a run and both happen to mount at once - not `position:fixed` from the viewport bottom, which used to float it over the timeline's FX lane once the timeline grew past ~74px tall, ux audit #17), inside its own `AnimatePresence` so it fades in/out (`y: 8 -> 0`, 0.16s) independently of the scrim. `title={model}` carries the full raw model id on hover. While `planning`, prefixes a small pulsing dot (`motion.span`, `opacity` `[0.35, 1, 0.35]` looped over 1.1s - an indeterminate indicator, since there's no step count yet) and shows `` `Asking ${shortModel || "the model"}… first runs can take a while — Esc to stop` `` where `shortModel = engineDisplayName(model)` (Task 11, ux audit #17: the pill used to print the full raw model id). Otherwise, text is `` `Directing · step ${progress.step}/${progress.total} — Esc to stop` `` once `progress` resolves, else `"Directing… — Esc to stop"`. Its own `pointerdown` calls `e.stopPropagation()` before `onCancel`, so clicking it doesn't ALSO fire the underlying scrim's handler.

### Notes

- Cancelling never aborts the step (or the planning fetch) already in flight - only what comes AFTER it is skipped: `useDirector.ts`'s `planOrCancel` still awaits the fetch to completion before checking `cancelRef`, and `reveal`'s loop only checks `cancelRef` at the top of each iteration, so the current step always finishes applying.
- The scrim/Escape/Stop-pill are the SAME cancel surface for both phases - only the copy (and, internally, which `useDirector` code path reacts to `cancelRef`) changes with `planning`.
