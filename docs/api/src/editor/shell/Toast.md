# src/editor/shell/Toast.tsx

A minimal, auto-dismissing feedback pill - currently fired only after undo/redo. `useUndoToast` owns its state so `Editor` just wires `onSwap` into `useEditHistory` and renders `<Toast>`.

## ToastMsg

```ts
export interface ToastMsg { id: number; text: string }
```

One toast instance: `id` is a monotonically-bumped counter (not the text) so `Toast`'s `AnimatePresence` key always changes on a new fire, even for the same `text` twice in a row - see `useUndoToast`.

## useUndoToast

```ts
export function useUndoToast(): { msg: ToastMsg | null; onSwap: (kind: "undo" | "redo") => void; dismiss: () => void }
```

### Returns

- `msg: ToastMsg | null` - the current toast, or `null` when nothing is showing. Pass straight to `Toast`'s `msg` prop.
- `onSwap: (kind: "undo" | "redo") => void` - bumps the internal id and sets `msg` to `{ id, text: "Undid" }` or `{ id, text: "Redid" }`. Wired to `useEditHistory`'s `onSwap` parameter, so it fires once per actual undo/redo swap (button click OR the hook's own Ctrl+Z listener) - never for a no-op undo/redo.
- `dismiss: () => void` - sets `msg` back to `null`. Pass to `Toast`'s `onDone` prop.

## Toast

```tsx
export function Toast({ msg, onDone }: { msg: ToastMsg | null; onDone: () => void }): JSX.Element
```

### Props

- `msg: ToastMsg | null` - what to show; renders nothing (`AnimatePresence` exit) when `null`.
- `onDone: () => void` - called `DISMISS_MS` (1.6s) after `msg` is set, via a `useEffect` timer that resets on every new `msg` (keyed by reference, so a fresh `onSwap` fire restarts the clock rather than the two dismissals racing).

### Behavior

Renders `msg.text` in an `.e-toast` pill (`--e-card` background, `--e-border`, `--e-shadow-pop`, 12.5px text), `AnimatePresence`-managed with `key={msg.id}` so a repeat fire (same text, new id) remounts and re-plays the entrance rather than sitting static. Motion: `opacity: 0 -> 1`, `y: 8 -> 0`, a `0.16s` tween (`ease: [0.4, 0, 0.2, 1]`) - the same entrance pattern used across the editor's other small transitions (`ShortcutsOverlay`, `ExportDialog`).

### Used by

`Editor` (`src/editor/Editor.tsx`) - calls `useUndoToast()`, wires its `onSwap` into `useEditHistory`, and renders `<Toast msg={toast} onDone={dismissToast} />` inside `.e-stagetoast`, the relative-positioned wrapper around `Stage` that anchors the pill bottom-center of the stage (see `editor.css`).
