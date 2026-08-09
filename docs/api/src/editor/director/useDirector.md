# src/editor/director/useDirector.ts

Owns the AI director's fake-pointer handle plus the run's live progress/planning/cancel state, and drives the choreographed reveal AROUND `Editor.tsx`'s existing queued-pass correctness machinery (the F2 fix, commit `d08c71e` - single `record(doc0)`, `running` set pre-enqueue, per-step raw `applyEditOp`/`setDoc`/`rev`, all inside ONE `enqueue(...)` call). Kept out of `Editor.tsx` (already at its line budget) so the choreography doesn't grow the file that owns undo/queue correctness - `run` performs the per-step body EXACTLY as the pre-Task-25 inline loop did, just from here instead. One deliberate ordering change since then (Task 40): `record(doc0)` no longer runs up front - it's deferred until immediately before `reveal` actually starts (via `fetchStepsToReveal` below), so a pass cancelled during planning never pushes a no-op snapshot onto the undo stack.

## planOrCancel

```ts
export function planOrCancel<T>(cancelRef: { current: boolean }, fetchPlan: () => Promise<T>): Promise<T | null>
```

Races a fetch against a cancel flag WITHOUT aborting it: awaits `fetchPlan()` to completion no matter what, then resolves to `null` (discarding the result) if `cancelRef.current` was set at any point before it settled, or to the fetched value otherwise. This is the seam `run` uses to make the `ai_plan` fetch window cancellable even though `ai_plan`'s Ollama HTTP call (server-side, can take minutes on a first-run model load per Task 40) has no cancellation of its own - Esc/scrim/Stop-pill during the fetch only ever flips `cancelRef`, never aborts the request. Plain function (no React), so this path is unit-tested directly in `useDirector.test.ts` without mounting the hook.

## fetchStepsToReveal

```ts
export function fetchStepsToReveal<T>(cancelRef: { current: boolean }, fetchPlan: () => Promise<T>, record: () => void): Promise<T | null>
```

Wraps `planOrCancel` with the undo-boundary fix: calls `record()` ONLY when `planOrCancel` actually produced steps (not cancelled, not rejected) - immediately before returning them. Fixes a bug where `run` used to call `record(doc0)` unconditionally, BEFORE the cancellable fetch: cancelling during a slow (first-run-model-load) planning fetch still left a no-op snapshot on the undo stack even though nothing was ever applied (the Undo button would light up over nothing - a redundant `setDoc`+`saveEdit` IPC write and an "Undid" toast for a pass that changed nothing). `record` never runs on a rejection either (it's inside the same `await`ed expression as the fetch - a throw from `fetchPlan()` propagates before `record` is reached). Plain function (no React) - unit-tested directly in `useDirector.test.ts`, including an ordering assertion (`record` fires strictly after the fetch resolves and before the caller's next step).

## DirectorProgress

```ts
export interface DirectorProgress { step: number; total: number }
```

The reveal's live position - `step` completed steps out of `total`. `Editor` threads this straight through to `AiPanel` (the "Directing… k of N" label + progress track) and `DirectorScrim` (the Stop pill's text).

## useDirector

```ts
export function useDirector(): {
  pointerRef: RefObject<DirectorPointerHandle | null>;
  progress: DirectorProgress | null;
  planning: boolean;
  cancelRef: RefObject<boolean>;
  requestCancel: () => void;
  run: (
    folder: string, docRef: RefObject<EditDoc | null>, record: (d: EditDoc) => void, dur: number,
    setDoc: (d: EditDoc) => void, setRev: Dispatch<SetStateAction<number>>, setAiLog: Dispatch<SetStateAction<string[]>>,
    setAiError: (e: string | null) => void, setPlaying: (p: boolean) => void, setTimeMs: (ms: number) => void,
  ) => Promise<void>;
}
```

### Returns

- `pointerRef` - handed to `DirectorPointer`'s `ref` (via `DirectorOverlay`); `reveal` (below) drives it.
- `progress` - see `DirectorProgress`. `null` before a run starts and while the plan is still being fetched.
- `planning` - `true` only while the `ai_plan` fetch itself is in flight (before `reveal` starts). `DirectorScrim` uses it to show "Asking `<model>`…" instead of the step-count text - `AiPanel` still only needs `progress` (unchanged).
- `cancelRef` - a plain boolean ref (no `AbortController` - every step is short, and the planning-phase fetch can't be aborted anyway), set by `requestCancel`. Checked in two places: `planOrCancel`, once the fetch resolves, and `reveal`'s loop, at the TOP of each iteration - so the step (or fetch) already in flight always finishes, and only what comes after it is skipped.
- `requestCancel: () => void` - sets `cancelRef.current = true`. Wired to `DirectorScrim`'s scrim/Escape/Stop-pill cancel path, active during BOTH the planning fetch and the reveal.
- `run(...)` - see Behavior below. Called from `Editor`'s `onRun`, inside its single `enqueue(...)` call.

### Behavior

**`run(folder, docRef, record, dur, setDoc, setRev, setAiLog, setAiError, setPlaying, setTimeMs)`.** Reads `doc0 = docRef.current` (bails if `null`), resets `cancelRef.current = false` (the ONE reset for the whole pass - both the fetch below and `reveal` share it; `reveal` itself no longer resets it, so a cancel that lands during the fetch survives into the reveal-phase check), and sets `planning` true. Fetches the plan through `fetchStepsToReveal(cancelRef, () => aiPlan(folder, doc0.settings.ai_model || undefined), () => record(doc0))` rather than a bare `await aiPlan(...)` followed by an unconditional `record(doc0)` - see `fetchStepsToReveal` above. **`record(doc0)` now fires ONLY as part of that call, and only once a reveal is actually about to happen** - not up front - so a cancelled-during-planning pass never pushes a snapshot onto the undo stack. Once the fetch settles, `planning` goes back to `false`. A rejection (e.g. no Ollama models installed, Ollama unreachable) is caught into `setAiError` and returns immediately - `record` never ran, `reveal` never runs. If `fetchStepsToReveal` instead resolved to `null` (a cancel landed during the fetch), `run` appends `"Stopped before planning finished"` to `aiLog` and returns WITHOUT calling `reveal`, applying any step, or ever having called `record` - the returned `Promise` still resolves normally, so `Editor`'s `enqueue(...)` (and its `.finally(() => setRunning(false))`) release immediately. Otherwise (`record(doc0)` has now run exactly once - the whole pass is one undo step), calls the internal `reveal(steps, dur, doc0, applyStep)` with an `applyStep` closure that performs the UNCHANGED per-step body: raw `applyEditOp` (no per-step `record` - the one call above covers the whole pass), `setDoc`, `rev` bump, push `step.label` onto `aiLog`, and for `add_zoom_full` steps, pause playback and scrub `timeMs` to `at_ms`. Once `reveal` returns the count of steps actually applied (`kept`), appends either `"Stopped — kept N edit(s)"` (`kept < steps.length`, i.e. cancelled mid-reveal) or the `"✓ Done · N zoom(s)"` summary (unchanged wording from before this task) to `aiLog`. Callers still own `setRunning(true/false)` and the re-entrancy guard themselves (see `Editor.md`'s `onRun`) - `run`'s own `finally` is the CALLER's `.finally(() => setRunning(false))`, not internal to this function.

**`reveal(steps, dur, docBefore, applyStep)` (internal, not exported).** Sets `progress` to `{step: 0, total: steps.length}` (does NOT reset `cancelRef` - `run` already did, before the planning fetch), re-measures `anchorPoint("wand")` and `moveTo`s there (`DirectorPointer` already snapped to it on mount, but the plan fetch this waited behind can take seconds - a resize in that window shouldn't leave the next step aimed at a stale button position), then `press()`es the pointer (per `DirectorPointerHandle`'s `press`) - "the run visibly starts from the button the user sees". Then, for each step (checking `cancelRef.current` at the top and returning `i` early if cancelled):

1. `planStep(step.op)` (`choreography.ts`) decides the choreography kind.
2. Re-queries `.e-tlbody` fresh (`document.querySelector` - never cached, so a mid-run resize can't desync the pointer) and computes `dwellSettle(steps.length)` for this step's timing.
3. `"aim-timeline"` - moves to `timelinePointForMs(track, plan.ms, dur, plan.lane)`, dwells, and presses (all via the internal `aimAndPress` helper) - all BEFORE the real apply.
4. `"drag-trim"` (only for `set_trim` ops) - compares the CURRENT doc's resolved trim (`resolveTrim(current.trim, dur)`) against the op's resolved target (`resolveTrim({in_ms, out_ms}, dur)`); for each endpoint that actually changed, aims at the handle's CURRENT position, dwells, presses, then glides (visual-only `moveTo`, no op yet) to the NEW position - `set_trim` is the ONLY step kind whose `applyStep` call can be preceded by more than one pointer leg.
5. `"sweep-lane"` (`clear_zooms`) - runs `pointerRef.current.sweep(...)` across the bottom-most `.e-zoomrow` CONCURRENTLY with `applyStep(step)` (`Promise.all`), per the spec's "sweep... while the op applies".
6. Every OTHER kind (including `"drag-trim"` and `"none"`) calls `applyStep(step)` alone, AFTER any legs from steps 3-4 have finished - so `set_trim`'s real op still applies exactly once, at the end of its glide(s).
7. For `"aim-timeline"` steps, settles onto the REAL new pill (`pillPoint(track, newDoc.zooms.at(-1).id)`) rather than the pre-apply guess, when one is found.
8. Sleeps `settleMs`, updates `current` to the step's returned doc, and bumps `progress` to `{step: i+1, total}`.

Returns `steps.length` once every step has applied without cancellation.

### Notes

- Every `pointerRef.current?.method()` call is optional-chained - if the pointer somehow isn't mounted yet (it should always be, by the time `reveal` runs past the plan-fetch `await`), the real edit logic still proceeds unaffected, just without the visual.
- `run`/`reveal`/`aimAndPress` are plain closures re-created each render (not `useCallback`) - `Editor` calls `director.run(...)` fresh each `onRun` invocation anyway, so memoizing them buys nothing.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `const director = useDirector();`; `onRun` calls `director.run(...)`; `DirectorOverlay` (rendered as a sibling of `.e-body`) receives `director.pointerRef`, `director.progress`, `director.planning`, `doc.settings.ai_model || undefined` (as `model`), and `director.requestCancel`; `EditorPanels`/`AiPanel` receive `director.progress` as `aiProgress`/`progress` (unchanged - `AiPanel` doesn't need `planning`, since its button already shows a bare "Directing…" whenever `progress` is `null`).
