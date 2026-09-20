# src/editor/hooks/doc/useTimelineActions.ts

The "add a region at the playhead" handlers shared by the Effects panel/Transport quick-add buttons and the canvas double-click (`zoomAt`), plus `splitAt` (Batch 4 T9), the Transport Split tool's and the B key's own handler. Kept out of `Editor` so it stays under the line limit (Task 36, mirrors `useTrimActions.ts`) - each applies its op then selects the newly created region so the inspector opens on it immediately, add + focus as one gesture (`splitAt` selects nothing when the op was a no-op, since there is no new region to focus).

## useTimelineActions

```ts
export function useTimelineActions(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  timeMsRef: RefObject<number>,
  docRef: RefObject<EditDoc | null>,
  setSel: (id: string | null) => void,
  setPlaying: (p: boolean) => void,
): { addZoom: () => Promise<void>; addSpotlight: () => Promise<void>; addMask: (kind: MaskKind) => Promise<void>; addCameraMove: () => Promise<void>; addText: (kind: TextKind) => Promise<void>; zoomAt: (x: number, y: number) => Promise<void>; splitAt: () => Promise<void> }
```

### Inputs

- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - persists the resulting `add_zoom` / `add_effect` / `add_camera_move` / `add_zoom_full` + `update_zoom` ops.
- `timeMsRef: RefObject<number>` - the current playhead position, as a ref (not a plain `timeMs: number` - render hygiene pass); every add lands `Math.round(timeMsRef.current)`, read at CALL time rather than closed over at build time.
- `docRef: RefObject<EditDoc | null>` - `Editor`'s `docRef` (bug-sweep-2 Task 8, M2). `addCameraMove` snapshots `docRef.current?.camera_moves` BEFORE calling `applyOp`, so it can find the just-added keyframe by diffing ids against the doc `applyOp` returns - see `pickAddedCameraMoveId` and `addCameraMove` below.
- `setSel: (id: string | null) => void` - selects the newly created region's id once `applyOp` resolves.
- `setPlaying: (p: boolean) => void` - `zoomAt` pauses playback first, so the new zoom doesn't animate away out from under the click.

### Returns

`useCallback`'d (render hygiene pass - deps are `applyOp`/`timeMsRef`/`docRef`/`setSel`/`setPlaying`, none of which change on a playhead tick - `docRef` itself is a stable ref object for the whole session), so all four stay referentially stable across a tick - `Stage`'s `onZoomAt`, `Transport`'s `onAddZoom`, and `EditorPanels`' `addZoom`/`addSpotlight`/`addCameraMove` props (all three components `React.memo`'d) need that to actually skip re-rendering for them.

- `addZoom: () => Promise<void>` - adds a 2s zoom at the playhead (`add_zoom`), selects it.
- `addSpotlight: () => Promise<void>` - adds a 2s spotlight effect at the playhead (`add_effect`), selects it.
- `addMask: (kind: MaskKind) => Promise<void>` - adds a **3s** mask of that kind at the playhead (the same `add_effect` op, with the kind passed through) and selects it, so the stage drag box appears on the thing that was just created. *Why 3s where the spotlight is 2s:* a spotlight follows the cursor and reads instantly, where a mask is placed and then sized, so it wants a little more room on the timeline to grab. The rect it starts with is the model's `MASK_SEED_RECT`, applied by the Rust op rather than sent from here.
- `addCameraMove: () => Promise<void>` - adds a centered camera-move keyframe at the playhead (`add_camera_move { x: 0.5, y: 0.5, size: 0.25 }`), selects it BY ID (`pickAddedCameraMoveId`, below) rather than by array position.
- `addText: (kind: TextKind) => Promise<void>` - adds a 3s text item of that kind at the playhead (`add_text`), selects it. Three seconds rather than the two every other element gets, because a text item has to be READ: a title with a 420 ms in and a 420 ms out has barely a second at full strength inside a two-second span. It is the only add that takes an argument, because `add_text` seeds the text, the sub, the size, the anchor and the style from the kind (`edit::ops::textops`), and picking the kind is the whole of what the four Add pills and the four drop types choose between.
- `zoomAt: (x: number, y: number) => Promise<void>` - the canvas double-click-to-zoom handler passed to `Stage`: pauses, adds a full zoom (`add_zoom_full`), then targets it at the clicked 0..1 screen point (`update_zoom` with `{ target: { fixed: { x, y } } }`) and selects it.
- `splitAt: () => Promise<void>` (Batch 4 T9) - a thin `useCallback` over `runSplitAt` (below): `applyOp`, the raw `timeMsRef.current`, `docRef.current?.clips ?? []` and `setSel` passed straight through, so the interesting logic can be called - and tested - without a hook context.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the whole returned object is spread into the shell props (`model/shellProps.ts`'s `...timeline`), so `addMask` and `addText` reach `Transport`, `EditorPanels` and the pill row without Editor naming them. `addZoom`, `addSpotlight`, `addText` and `splitAt` are ALSO named individually in the `useEditorKeymap` call (the Z, S, T and B shortcuts), which is the one place they are not spread; `zoomAt` is passed to `Stage` as `onZoomAt`. `addMask` has no keyboard shortcut in this batch, so it is not passed to the keymap. `splitAt` also reaches `Transport`'s Split tool through `model/shellProps.ts`'s `onSplit: () => void timeline.splitAt()` (`shellProps.md`), a voided rename rather than a spread, because the tool wants a plain synchronous click handler under its own name.

## pickAddedCameraMoveId

```ts
export function pickAddedCameraMoveId(before: CameraMove[], after: CameraMove[]): string | null
```

Pure, unit-tested (`useTimelineActions.test.ts`) helper for `addCameraMove` (bug-sweep-2 Task 8, M2). Returns the id of the ONE entry present in `after` but not `before` (or `null` if none), through the private `addedSince` id-diff this file's other array-position bug fix, `pickSplitClipId` (below), now shares (fix round 1) rather than duplicates. *Why not `after[after.length - 1]`, like `addZoom`/`addSpotlight` use:* Rust's `AddCameraMove` SORTS `doc.camera_moves` by `t_ms` after pushing the new one (`api.rs`) - `add_zoom`/`add_effect`/`add_layout_seg` stay genuinely append-only, so `[length - 1]` is correct for THOSE, but for camera moves the new entry's index moves the instant it lands anywhere but the very end of the track. A keyframe added earlier than an existing one used to open the WRONG keyframe's inspector, silently misdirecting every subsequent edit (including Delete) at it.

## pickSplitClipId

```ts
export function pickSplitClipId(before: Clip[], after: Clip[], atMs: number): string | null
```

Pure, unit-tested (through `runSplitAt` below, in `useTimelineActions.test.ts`) helper for `splitAt`. Picks the clip a `split_at` created by the same id-diff as `pickAddedCameraMoveId` (`addedSince`), not by reading `after[after.length - 1]` - the array-position read Batch 4 T9 shipped selected the wrong clip whenever a split landed earlier than the doc's last clip, because Rust's `split` (`edit::ops::clipops`) inserts the new right-hand half at `i + 1`, not at the array's end, in every case except splitting an empty clip list (fix round 1).

Exactly one clip is ever newly created, except the very first split of a clip-less document, which materialises both halves - `lo..at_ms` and `at_ms..hi` - at once. So: no new id means the op no-opped (a clip edge or outside every clip) and this returns `null`; one new id is unambiguous and is returned as-is; two new ids means the first-ever split, and this returns whichever of the two has `src_in_ms === atMs` - the right-hand half, since a freshly created clip's `src_in_ms` is always where the cut fell. `Rust` guarantees one of the two always matches, so the trailing `created[created.length - 1]` is a defensive fallback that today's Rust never actually reaches.

## runSplitAt

```ts
export async function runSplitAt(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  atMs: number,
  clipsBefore: Clip[],
  setSel: (id: string | null) => void,
): Promise<void>
```

The plain (non-hook) core of `splitAt` (fix round 1), pulled out of the `useCallback` so it can be called - and its regression pinned - directly in a test, without a React render: `useCallback`/`useEffect`/etc. throw when called outside one, so nothing in this file could otherwise exercise the exact `applyOp` call and the exact selection together. Rounds `atMs` once, applies `{ op: "split_at", at_ms }`, then hands the returned doc's `clips` to `pickSplitClipId` against the `clipsBefore` snapshot and calls `setSel` only when it returns an id - so, like every no-op path in this file, a split that changed nothing leaves the selection exactly as it was.
