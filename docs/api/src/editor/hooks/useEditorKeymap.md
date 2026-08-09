# src/editor/hooks/useEditorKeymap.ts

Global keyboard shortcuts for the editor: Delete/Backspace removes the selected zoom/effect/layout segment/camera-move keyframe, Z/S add a zoom/spotlight at the playhead, Space toggles play, `?` opens the shortcuts overlay. Skipped while an input/textarea/contenteditable is focused. The actual key -> action decision is `keyAction` (`keymap.ts`, unit-tested); this hook just switches on its result and performs the corresponding op.

## useEditorKeymap

```ts
export function useEditorKeymap(opts: {
  sel: string | null;
  doc: EditDoc | null;
  timeMs: number;
  setSel: (id: string | null) => void;
  setPlaying: (fn: (p: boolean) => boolean) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  addZoom: () => Promise<void>;
  addSpotlight: () => Promise<void>;
  onOverlay: () => void;
}): void
```

### Inputs

- `sel: string | null` - the currently selected zoom/effect/layout/camera-move id, or `null`.
- `doc: EditDoc | null` - the edit doc; used to figure out WHICH region list `sel` belongs to before dispatching the matching `remove_*` op.
- `timeMs: number` - unused inside the handler itself, but in the effect's dependency array (kept alongside `sel`/`doc`) so the closures captured by `window.addEventListener` stay reasonably fresh - matches the hook's pre-existing minimal-deps convention (`addZoom`/`addSpotlight`/`applyOp`/`setSel`/`setPlaying`/`onOverlay` are NOT deps; the listener re-subscribes only on `sel`/`doc`/`timeMs` changes).
- `setSel`, `setPlaying`, `applyOp`, `addZoom`, `addSpotlight` - as before.
- `onOverlay: () => void` - called (with `e.preventDefault()`) when `keyAction` returns `"overlay"`; `Editor.tsx` wires this to toggle `showShortcuts` for `ShortcutsOverlay`.

### Returns

`void` - side-effect only; attaches a `window` `keydown` listener on mount, removes it on unmount/dep-change.

### Behavior

On every `keydown` (skipped while an input/textarea/contenteditable is focused), calls `keyAction(e, !!sel)` and switches on the result:

- `"delete"` - looks up which of `doc.zooms`/`doc.effects`/`doc.layout`/`doc.camera_moves` contains `sel` and dispatches the matching `remove_zoom`/`remove_effect`/`remove_layout_seg`/`remove_camera_move`, then clears `sel`. If `sel` doesn't match anything currently in the doc (stale selection), no-ops without touching `sel`.
- `"zoom"` / `"spotlight"` - calls `addZoom()` / `addSpotlight()`.
- `"play"` - `e.preventDefault()` then toggles `playing`.
- `"overlay"` - `e.preventDefault()` then calls `onOverlay()`.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
