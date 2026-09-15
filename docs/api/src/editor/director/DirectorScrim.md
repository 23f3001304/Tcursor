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

- `running: boolean` - whether an AI-director pass is in flight (`useAiRun`'s `running`: thinking, applying or replaying). Both the scrim and the Stop pill are mounted only while this is true.
- `planning: boolean` - `useAiRun`'s `planning` - `true` only while the propose fetch is in flight, `false` once it settles (whether it resolved, rejected, or was cancelled). Switches the Stop pill from its thinking copy to the replay's step-count copy.
- `model?: string` - `doc.settings.ai_model || undefined`, for the thinking pill's `"Thinking with <model>…"` text. Falls back to `"the model"` when unset (auto-pick).
- `progress: { step: number; total: number } | null` - the replay's live position, from `useAiRun`'s `progress`. `null` while thinking and while applying; non-null IS "the replay is walking".
- `onCancel: () => void` - `useAiRun`'s `requestCancel` - sets the `cancelRef` that `planOrCancel` (during thinking) and the replay loop check.

### Behavior

**Scrim.** A transparent, full-viewport `.e-director-scrim` (`pointer-events: all`, NO visual dim - deliberately invisible, it only exists to catch input) that calls `onCancel` on ANY `pointerdown`. Mounted/unmounted directly on `running` (no exit animation - it should disappear the instant the run ends, not fade).

**Escape.** A `keydown` listener, attached only while `running` (removed the moment it flips false), calls `onCancel` on `Escape`.

**Stop pill.** A `motion.div.e-director-stop` (Task 11: now positioned `position:absolute` inside `Editor.tsx`'s `.e-stagetoast` - the same stage-bottom container `Toast` anchors in, `bottom: 60px` vs. `Toast`'s own `22px` so the two stack rather than overlap if an undo/redo is queued during a run and both happen to mount at once - not `position:fixed` from the viewport bottom, which used to float it over the timeline's FX lane once the timeline grew past ~74px tall, ux audit #17), inside its own `AnimatePresence` so it fades in/out (`y: 8 -> 0`, 0.16s) independently of the scrim. `title={model}` carries the full raw model id on hover. The pill is text only now: the pulsing dot it used to prefix while `planning` was replaced by `DirectorOverlay`'s sweeping wave just above it, which carries the pass's progress in every phase (indeterminate while thinking and applying, a real head position once the replay walks). While `planning` it shows `` `Thinking with ${shortModel || "the model"}… first runs can take a while - Esc to stop` `` where `shortModel = engineDisplayName(model)` (Task 11, ux audit #17: the pill used to print the full raw model id). Otherwise, text is `` `Replaying · ${progress.step}/${progress.total} - Esc to stop` `` while `progress` is non-null, else `"Applying…"` (the short apply between the two, which cannot be cancelled). Its own `pointerdown` calls `e.stopPropagation()` before `onCancel`, so clicking it doesn't ALSO fire the underlying scrim's handler.

The pill wraps and is capped at `min(420px, calc(100% - 32px))` like `.e-toast` (width audit, 2026-09-14). Its label is a whole sentence, which as one `nowrap` line is about 340px - wider than the stage on a narrow window, where a `fit-content` pill spilled sideways over the timeline and the sidebar. Its radius drops from a 999px capsule to the panel radius for the same reason: a two-line pill with capsule ends reads as a mistake.

### Notes

- Cancelling never aborts what is already in flight - only what comes AFTER it is skipped: `useAiRun.ts`'s `planOrCancel` still awaits the propose fetch to completion before checking `cancelRef` (and then discards the answer), and the replay loop only checks `cancelRef` at the top of each step. The apply itself is never cancelled: it is one undo step and must land whole. Cancelling the replay never touches the edits, which are already applied.
- The scrim/Escape/Stop-pill are the SAME cancel surface for every phase - only the copy (and, internally, which `useAiRun` code path reacts to `cancelRef`) changes.
