# src/editor/hooks/useTimelineActions.ts

The "add a region at the playhead" handlers shared by the Effects panel/Transport quick-add buttons and the canvas double-click (`zoomAt`). Kept out of `Editor` so it stays under the line limit (Task 36, mirrors `useTrimActions.ts`) - each applies its op then selects the newly created region so the inspector opens on it immediately, add + focus as one gesture.

## useTimelineActions

```ts
export function useTimelineActions(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  timeMsRef: RefObject<number>,
  docRef: RefObject<EditDoc | null>,
  setSel: (id: string | null) => void,
  setPlaying: (p: boolean) => void,
): { addZoom: () => Promise<void>; addSpotlight: () => Promise<void>; addCameraMove: () => Promise<void>; zoomAt: (x: number, y: number) => Promise<void> }
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
- `addCameraMove: () => Promise<void>` - adds a centered camera-move keyframe at the playhead (`add_camera_move { x: 0.5, y: 0.5, size: 0.25 }`), selects it BY ID (`pickAddedCameraMoveId`, below) rather than by array position.
- `zoomAt: (x: number, y: number) => Promise<void>` - the canvas double-click-to-zoom handler passed to `Stage`: pauses, adds a full zoom (`add_zoom_full`), then targets it at the clicked 0..1 screen point (`update_zoom` with `{ target: { fixed: { x, y } } }`) and selects it.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `addZoom`/`addSpotlight`/`addCameraMove` are passed to `Transport`, `EditorPanels`, and `useEditorKeymap` (Z/S shortcuts); `zoomAt` is passed to `Stage` as `onZoomAt`.

## pickAddedCameraMoveId

```ts
export function pickAddedCameraMoveId(before: CameraMove[], after: CameraMove[]): string | null
```

Pure, unit-tested (`useTimelineActions.test.ts`) helper for `addCameraMove` (bug-sweep-2 Task 8, M2). Returns the id of the ONE entry present in `after` but not `before` (or `null` if none). *Why not `after[after.length - 1]`, like `addZoom`/`addSpotlight` use:* Rust's `AddCameraMove` SORTS `doc.camera_moves` by `t_ms` after pushing the new one (`api.rs`) - `add_zoom`/`add_effect`/`add_layout_seg` stay genuinely append-only, so `[length - 1]` is correct for THOSE, but for camera moves the new entry's index moves the instant it lands anywhere but the very end of the track. A keyframe added earlier than an existing one used to open the WRONG keyframe's inspector, silently misdirecting every subsequent edit (including Delete) at it.
