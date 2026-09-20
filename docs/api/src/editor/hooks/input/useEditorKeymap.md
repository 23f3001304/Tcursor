# src/editor/hooks/input/useEditorKeymap.ts

Global keyboard shortcuts for the editor: Delete/Backspace removes the selected zoom/effect/layout segment/camera-move keyframe, Z/S/T add a zoom/spotlight/text item at the playhead, B (Batch 4 T9) splits the clip at the playhead, Space toggles play, `?` opens (or, while it's the current modal, closes) the shortcuts overlay. Inert while typing in a field, while a modal is open (`modalOpen`), or - for Space specifically - while the focused element owns Space itself (a button, or a custom control like `Switch`/`Picker` that manages its own keydown; bug-sweep-2 Task 8, M4). The actual decision is `resolveKeyAction` (`keymap.ts`, unit-tested); this hook just gathers the DOM context and switches on its result.

It is NOT the only consumer of that decision: M1a's `shell/frame/useMaximize.ts` reads the same function for `Ctrl+Space` ("maximize"), from a capture-phase listener that runs first. This hook has no case for that action and ignores it.

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
  addText: (kind: TextKind) => Promise<void>;
  splitAt: () => Promise<void>;
  onOverlay: () => void;
  modalOpen: boolean;
  shortcutsOpen: boolean;
}): void
```

### Inputs

- `sel: string | null` - the currently selected zoom/effect/layout/camera-move id, or `null`.
- `doc: EditDoc | null` - the edit doc; used to figure out WHICH region list `sel` belongs to before dispatching the matching `remove_*` op. The `delete` arm walks zooms, effects, layout segments, camera moves, cuts, speed spans and texts in that order, and `doc.texts` is read with `?.` because a document written before Batch 1 has no such list.
- `timeMs: number` - unused inside the handler itself, but in the effect's dependency array (kept alongside `sel`/`doc`/`modalOpen`/`shortcutsOpen`) so the closures captured by `window.addEventListener` stay reasonably fresh - matches the hook's pre-existing minimal-deps convention (`addZoom`/`addSpotlight`/`applyOp`/`setSel`/`setPlaying`/`onOverlay` are NOT deps; the listener re-subscribes only on `sel`/`doc`/`timeMs`/`modalOpen`/`shortcutsOpen`/`escOwned` changes).
- `setSel`, `setPlaying`, `applyOp`, `addZoom`, `addSpotlight` - as before.
- `addText: (kind: TextKind) => Promise<void>` - `useTimelineActions`' own `addText`. The `"text"` action calls it with `"title"`: one key cannot choose between four kinds, so the shortcut takes the default and the Add pills take the choice.
- `splitAt: () => Promise<void>` (Batch 4 T9) - `useTimelineActions`' own `splitAt`. The `"split"` action calls it directly; unlike `addZoom`/`addSpotlight`/`addText` it may do nothing at all (a split at a clip edge or outside every clip is a no-op), which `splitAt` itself decides, not this hook.
- `onOverlay: () => void` - called (with `e.preventDefault()`) when `resolveKeyAction` returns `"overlay"`; `Editor.tsx` wires this to toggle `showShortcuts` for `ShortcutsOverlay`.
- `modalOpen: boolean` (bug-sweep-2 Task 8, M4; extended in review round 1) - `Editor.tsx`'s `showExportDialog || showShortcuts || showSettings || moveOffOpen || running`. Every shortcut is inert while this is true, so e.g. `z` typed while the Export dialog is open (or the AI director is running) can no longer silently add a zoom (and an undo step) behind it.
- `shortcutsOpen: boolean` (review round 1 minor) - `Editor.tsx`'s `showShortcuts`, passed SEPARATELY from `modalOpen` (even though it's also one of the terms that makes `modalOpen` true) so `resolveKeyAction` can still let `?` through to CLOSE the overlay while it, specifically, is the open modal - see `keymap.md`'s `resolveKeyAction`.
- `escOwned: boolean` (M1a) - true while a live surface owns Escape for its own dismiss. `Editor.tsx` passes `arrangeOn`: stage arrange mode's local Escape listener exits the mode and deliberately KEEPS the segment selected, so the pill and `LayoutInspector`'s button can re-enter (`useArrangeMode.md`). Without this flag the new global `"deselect"` action would fire on the same keydown and drop that selection, turning one Escape into two different exits. It is a WITHHOLD, not a re-route: the action is still resolved, `useEditorKeymap` just does not act on it.

### Returns

`void` - side-effect only; attaches a `window` `keydown` listener on mount, removes it on unmount/dep-change.

### Behavior

On every `keydown`, reads `document.activeElement` into a `TargetLike` (`tagName`/`role`/`isContentEditable`) and calls `resolveKeyAction(e, { hasSel: !!sel, modalOpen, shortcutsOpen, target })` (`keymap.ts`), then switches on the result:

- `"delete"` - looks up which of `doc.zooms`/`doc.effects`/`doc.layout`/`doc.camera_moves`/`doc.cuts`/`doc.speed` contains `sel` and dispatches the matching `remove_zoom`/`remove_effect`/`remove_layout_seg`/`remove_camera_move`/`remove_cut`/`remove_speed`, then clears `sel`. If `sel` doesn't match anything currently in the doc (stale selection), no-ops without touching `sel`. The last two arrived with the time remap (T7): a cut is selected by clicking its hatched span (`timeline/lanes/CutOverlay.tsx`), a speed span by clicking its pill on the Time lane, and both are removed here rather than by a listener of their own - one delete path, so a new region kind is one arm on this ladder.
- `"deselect"` - clears `sel`, unless `escOwned` says another surface is currently the meaning of Escape.
- `"zoom"` / `"spotlight"` - calls `addZoom()` / `addSpotlight()`.
- `"text"` (Batch 2c) - calls `addText("title")`.
- `"split"` (Batch 4 T9) - calls `splitAt()`.
- `"play"` - `e.preventDefault()` then toggles `playing`. `resolveKeyAction` already withheld this entirely when the focused control owns Space itself, so this `preventDefault()` can no longer suppress e.g. a `Switch`'s own native activation.
- `"overlay"` - `e.preventDefault()` then calls `onOverlay()`.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
