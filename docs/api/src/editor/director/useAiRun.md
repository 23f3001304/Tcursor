# src/editor/director/useAiRun.ts

The AI Director's whole client side, in one hook: the propose pass, the review sheet's state and its handlers, the one-undo-step apply, and the optional pointer replay after it. It replaced the one-shot reveal hook (which applied a plan step by step, narrating as it went) and the sheet-state hook that T4 had split out to wait for it, in M4 T5.

**Three phases, one `running`.** `planning` while the model thinks (`aiPropose`), `applying` while the accepted ops land, `replaying` while the fake pointer performs them. `running` is any of the three: it disables the wand and the run button, mounts `DirectorOverlay` (the pointer, the wave, the scrim and the Stop pill) and puts the brand mark into its directing state. Only `applying` holds `Editor`'s op queue; thinking and replaying do not, so the user can keep editing while the model works, and a mid-apply undo queues behind the apply instead of racing it.

**Inputs through a ref.** `useAiRun(io)` stores `io` in a ref and re-assigns it every render, so every callback it returns has one identity for the editor's lifetime and reads `folder` / `dur` / `docRef` / `record` fresh at call time. Those callbacks travel down `ShellProps` into `EditorPanels`, which is `React.memo`'d; an identity that changed each render would re-render the panel column on every playhead tick.

## planOrCancel

```ts
export async function planOrCancel<T>(cancelRef: { current: boolean }, fetch: () => Promise<T>): Promise<T | null>
```

Races `fetch` against a cancel flag WITHOUT aborting it: awaits the fetch to completion, then resolves to `null` (discarding the result) if `cancelRef.current` was set at any point before it settled, or to the fetched value otherwise. The propose pass's HTTP call has no cancellation of its own (a first-run model load can take minutes), so Esc, the scrim and the Stop pill only ever flip the flag; this is where the app side stops reacting. A rejected fetch rejects as-is. Plain function, unit-tested in `useAiRun.test.ts` without mounting the hook.

## Rect

```ts
export type Rect = [number, number, number, number]
```

A region in 0..1 of the screen content, `[x, y, w, h]`: the shape a proposal's `rect` and the stage outline share.

## AiProgress

```ts
export interface AiProgress { step: number; total: number }
```

The replay's live position: how many applied proposals the pointer has performed, of how many it will.

## AiRunIo

```ts
export interface AiRunIo {
  folder: string;
  docRef: RefObject<EditDoc | null>;
  dur: number;
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>;
  record: (d: EditDoc) => void;
  setDoc: (d: EditDoc) => void;
  bumpRev: () => void;
  onSeek: (ms: number) => void;
  setPlaying: (p: boolean) => void;
}
```

What the hook needs from `Editor.tsx`: the project folder, the live doc ref, the clip length (for aiming the replay), the one op queue (`opQueue.ts`), the undo `record`, the doc setter, a `rev` bump (so the camera curve refetches once after an apply), and the seek and play controls the preview uses.

## useAiRun

```ts
export function useAiRun(io: AiRunIo): {
  pointerRef: RefObject<DirectorPointerHandle | null>; cancelRef: { current: boolean }; requestCancel: () => void;
  run: () => Promise<void>; apply: () => Promise<void>; discard: () => void;
  toggleItem: (id: string) => void; preview: (id: string) => void;
  aiRun: AiRun | null; skipped: ReadonlySet<string>; previewId: string | null; outline: Rect | null;
  planning: boolean; applying: boolean; replaying: boolean; progress: AiProgress | null; error: string | null;
  running: boolean;
}
```

### Behavior

**`run()`, the propose pass.** Refuses re-entry (a synchronous `busyRef`, since the `planning` state lags a render), clears the cancel flag and the last error, then `planOrCancel(cancelRef, () => aiPropose(folder, doc.settings.ai_model || undefined))`. A result replaces the sheet's state wholesale: the new run, an empty skip set (every proposal starts accepted), no preview, no outline. `null` (cancelled) leaves the sheet as it was. A rejection lands in `error`, which `AiPanel` shows with a Retry.

**`toggleItem(id)` / `preview(id)` / `discard()`, the sheet.** Toggle flips one id in the skip set through `reviewState.toggle` (a new set each time, so the row re-renders). Preview stops playback first, then seeks to the proposal's `at_ms` and sets `outline` from its `rect` (`reviewState.previewTarget`): a preview that immediately scrolls away from the thing it points at is not a preview. The outline clears itself after `OUTLINE_MS` (2500ms) and takes `previewId` with it; it marks a moment, not a mode. Discard resets all four pieces of state at once.

**`apply()`, one undo step.** Refuses re-entry the same way, then runs `applyRun(aiRun, skipped, io)` (`applyOps.md`) inside `io.enqueue`, with `applyEditOp(folder, op)` as the raw edit call. `applyRun` records the pre-apply doc exactly once and only when something is accepted, so a skip-everything Apply pushes no undo entry. After it resolves: one `bumpRev()` if anything landed (not per op), then `discard()` clears the sheet, since its job is done. A rejection lands in `error`; the ops that landed before it stay, under the one undo step.

**The replay, only by request.** After a successful apply, and only when `docRef.current.settings.ui.ai_choreography` is true (`InterfaceSettings`, off by default), the hook walks the accepted proposals with the fake pointer: a first dwell so the pointer's mount fade finishes (it mounted when `applying` went true, through `DirectorOverlay`), a press on the wand anchor, then per proposal `planStep(proposal.ops[0])` (`choreography.md`): move to `timelinePointForMs(track, plan.ms, dur, plan.lane)`, dwell, press, settle. A `"none"` plan (a layout segment, a cut, a speed span) is skipped. `progress` counts proposals, `dwellSettle` paces them. It applies nothing. Cancelling (`requestCancel`) stops the walk after its current step; the edits are already in the doc and stay.

**`requestCancel()`.** Sets `cancelRef.current`. During planning it discards the answer when it arrives; during the replay it stops the pointer; during the apply it does nothing (the apply is short and must finish as one step).

### Used by

- `src/editor/Editor.tsx` - one call; its fields spread into `ShellProps` (`running`, `aiError`, `aiRun`, `aiSkipped`, `aiApplying`, `aiPreviewId`, `stageOutline`, `aiProgress`, `aiPlanning`, `pointerRef` and the handlers), and `run` behind the wand's `onRun`.
