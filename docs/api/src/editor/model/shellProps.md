# src/editor/model/shellProps.ts

The adapter between the editor's hook bundles and the shell's slot contract. `ClassicShell` takes one `ShellProps` object of about ninety fields; `Editor` holds them across eight bundles (the session, the callbacks, arrange mode, the timeline actions, the trim actions, the AI run, the view state, and the doc itself). This file is the one place that map is written down, so `Editor.tsx` stays a component instead of ending in a ninety-line object literal.

## ShellView

```ts
export interface ShellView { sel, setSel, tab, setTab, range, setRange, timeMs, timeMsRef,
  muted, volume, setVolume, quality, modalOpen, trimmed, aimMode, aimPoint, setAimOn,
  moveMode, camDraftRef, onDuration, onDetectSilences, onRun }
```

The part of `ShellProps` that is `Editor`'s own view state rather than any hook's return - named as a type so the component cannot quietly drift out of step with what the shell needs. The setter fields are `Dispatch<SetStateAction<T>>`, matching `SlotProps` exactly, because several panels call them with an updater function.

## buildShellProps

```ts
export function buildShellProps(a: { folder, doc, session, cb, arrange, timeline, trim, ai, view }): ShellProps
```

Assembles the bundle. The five plain bundles (`cb`, `arrange`, `timeline`, `trim`, `view`) spread in wholesale - their field names ARE the shell's, deliberately, so the map stays a spread rather than thirty renames. `session` and `ai` are read field by field, because both return more than the shell wants (`session` also carries `docRef`, `enqueue`, `record` and the raw setters; `ai` names its fields for the director, not for the slots) and because two shell fields are derived here: `webcamSrc` from the folder and `aspectLocked` from `exporting || dur <= 0`. The argument types are `ReturnType<typeof useX>` on purpose - re-declaring ninety field types to hand them across one function call would be the exact duplication this file exists to avoid.

### Notes

**The `shell` bundle.** Everything the four editor types need reaches `EditorShell` as one `ShellProps` object (`shell/slotProps.md`), built here. That is what let `Editor.tsx` drop back under the 200-line cap while gaining a whole layout subsystem: the ~45 lines of JSX attributes it used to spend placing five components became ~12 lines of object literal, and `EditorSlot` spreads them back out.

**The AI Director (M4 T5).** `const ai = useAiRun({ folder, docRef, dur, enqueue, record, setDoc, bumpRev, onSeek, setPlaying })` (`director/useAiRun.md`) owns the whole client side: the propose pass, the review sheet's state (the run, the skipped ids, the previewed id, the stage outline) and its handlers, the one-undo-step apply, and the optional pointer replay. Its fields spread into `ShellProps` as `running` (`ai.running`: thinking, applying or replaying), `aiError`, `aiRun` / `aiSkipped` / `aiApplying` / `aiPreviewId` / `stageOutline`, `aiProgress` / `aiPlanning` / `pointerRef` / `onCancelRun`, and the handlers `onToggleItem` / `onPreviewItem` / `onApplyRun` / `onDiscardRun`. The hook reads its inputs through a ref at call time, so every handler keeps one identity across ticks without `Editor` juggling deps; `bumpRev` is a fresh arrow each render for the same reason it can be: nothing depends on its identity.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
