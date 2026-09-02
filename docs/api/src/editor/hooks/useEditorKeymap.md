# src/editor/hooks/useEditorKeymap.ts

Global keyboard shortcuts for the editor: Delete/Backspace removes the selected zoom/effect/layout segment/camera-move keyframe, Z/S add a zoom/spotlight at the playhead, Space toggles play, `?` opens (or, while it's the current modal, closes) the shortcuts overlay. Inert while typing in a field, while a modal is open (`modalOpen`), or - for Space specifically - while the focused element owns Space itself (a button, or a custom control like `Switch`/`Picker` that manages its own keydown; bug-sweep-2 Task 8, M4). The actual decision is `resolveKeyAction` (`keymap.ts`, unit-tested); this hook just gathers the DOM context and switches on its result.

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
  modalOpen: boolean;
  shortcutsOpen: boolean;
}): void
```

### Inputs

- `sel: string | null` - the currently selected zoom/effect/layout/camera-move id, or `null`.
- `doc: EditDoc | null` - the edit doc; used to figure out WHICH region list `sel` belongs to before dispatching the matching `remove_*` op.
- `timeMs: number` - unused inside the handler itself, but in the effect's dependency array (kept alongside `sel`/`doc`/`modalOpen`/`shortcutsOpen`) so the closures captured by `window.addEventListener` stay reasonably fresh - matches the hook's pre-existing minimal-deps convention (`addZoom`/`addSpotlight`/`applyOp`/`setSel`/`setPlaying`/`onOverlay` are NOT deps; the listener re-subscribes only on `sel`/`doc`/`timeMs`/`modalOpen`/`shortcutsOpen` changes).
- `setSel`, `setPlaying`, `applyOp`, `addZoom`, `addSpotlight` - as before.
- `onOverlay: () => void` - called (with `e.preventDefault()`) when `resolveKeyAction` returns `"overlay"`; `Editor.tsx` wires this to toggle `showShortcuts` for `ShortcutsOverlay`.
- `modalOpen: boolean` (bug-sweep-2 Task 8, M4; extended in review round 1) - `Editor.tsx`'s `showExportDialog || showShortcuts || showSettings || moveOffOpen || running`. Every shortcut is inert while this is true, so e.g. `z` typed while the Export dialog is open (or the AI director is running) can no longer silently add a zoom (and an undo step) behind it.
- `shortcutsOpen: boolean` (review round 1 minor) - `Editor.tsx`'s `showShortcuts`, passed SEPARATELY from `modalOpen` (even though it's also one of the terms that makes `modalOpen` true) so `resolveKeyAction` can still let `?` through to CLOSE the overlay while it, specifically, is the open modal - see `keymap.md`'s `resolveKeyAction`.

### Returns

`void` - side-effect only; attaches a `window` `keydown` listener on mount, removes it on unmount/dep-change.

### Behavior

On every `keydown`, reads `document.activeElement` into a `TargetLike` (`tagName`/`role`/`isContentEditable`) and calls `resolveKeyAction(e, { hasSel: !!sel, modalOpen, shortcutsOpen, target })` (`keymap.ts`), then switches on the result:

- `"delete"` - looks up which of `doc.zooms`/`doc.effects`/`doc.layout`/`doc.camera_moves` contains `sel` and dispatches the matching `remove_zoom`/`remove_effect`/`remove_layout_seg`/`remove_camera_move`, then clears `sel`. If `sel` doesn't match anything currently in the doc (stale selection), no-ops without touching `sel`.
- `"zoom"` / `"spotlight"` - calls `addZoom()` / `addSpotlight()`.
- `"play"` - `e.preventDefault()` then toggles `playing`. `resolveKeyAction` already withheld this entirely when the focused control owns Space itself, so this `preventDefault()` can no longer suppress e.g. a `Switch`'s own native activation.
- `"overlay"` - `e.preventDefault()` then calls `onOverlay()`.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
