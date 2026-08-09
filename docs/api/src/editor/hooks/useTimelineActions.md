# src/editor/hooks/useTimelineActions.ts

The "add a region at the playhead" handlers shared by the Rail/Transport quick-add buttons and the canvas double-click (`zoomAt`). Kept out of `Editor` so it stays under the line limit (Task 36, mirrors `useTrimActions.ts`) - each applies its op then selects the newly created region so the inspector opens on it immediately, add + focus as one gesture.

## useTimelineActions

```ts
export function useTimelineActions(
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  timeMs: number,
  setSel: (id: string | null) => void,
  setPlaying: (p: boolean) => void,
): { addZoom: () => Promise<void>; addSpotlight: () => Promise<void>; addCameraMove: () => Promise<void>; zoomAt: (x: number, y: number) => Promise<void> }
```

### Inputs

- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - persists the resulting `add_zoom` / `add_effect` / `add_camera_move` / `add_zoom_full` + `update_zoom` ops.
- `timeMs: number` - the current playhead position; every add lands `Math.round(timeMs)`.
- `setSel: (id: string | null) => void` - selects the newly created region's id once `applyOp` resolves.
- `setPlaying: (p: boolean) => void` - `zoomAt` pauses playback first, so the new zoom doesn't animate away out from under the click.

### Returns

Fresh closures each render - no React state of its own.

- `addZoom: () => Promise<void>` - adds a 2s zoom at the playhead (`add_zoom`), selects it.
- `addSpotlight: () => Promise<void>` - adds a 2s spotlight effect at the playhead (`add_effect`), selects it.
- `addCameraMove: () => Promise<void>` - adds a centered camera-move keyframe at the playhead (`add_camera_move { x: 0.5, y: 0.5, size: 0.25 }`), selects it.
- `zoomAt: (x: number, y: number) => Promise<void>` - the canvas double-click-to-zoom handler passed to `Stage`: pauses, adds a full zoom (`add_zoom_full`), then targets it at the clicked 0..1 screen point (`update_zoom` with `{ target: { fixed: { x, y } } }`) and selects it.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `addZoom`/`addSpotlight`/`addCameraMove` are passed to `Transport`, `EditorPanels`, and `useEditorKeymap` (Z/S shortcuts); `zoomAt` is passed to `Stage` as `onZoomAt`.
