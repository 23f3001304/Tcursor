# src/editor/hooks/keymap.ts

Pure key->action decision extracted from `useEditorKeymap` so the modifier/repeat guards are unit-testable without a DOM `KeyboardEvent` or a mounted hook.

## KeyLike

```ts
export interface KeyLike { key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; repeat: boolean }
```

The minimal shape `keyAction` reads off a `KeyboardEvent` - a plain interface (not the DOM type) so tests can pass object literals directly.

## keyAction

```ts
export type KeyAction = "delete" | "zoom" | "spotlight" | "play" | "overlay" | null;

export function keyAction(e: KeyLike, hasSel: boolean): KeyAction
```

Decides which editor shortcut (if any) one keydown triggers.

### Inputs

- `e: KeyLike` - the keydown's `key`/`ctrlKey`/`metaKey`/`altKey`/`repeat`.
- `hasSel: boolean` - whether something is currently selected in the timeline; gates `"delete"`.

### Returns

`KeyAction` - `null` when nothing should fire.

### Behavior

- **Modifier guard.** Returns `null` whenever `ctrlKey || metaKey || altKey` is true - unconditionally, before anything else. This is the actual bug fix: `z`/`s` previously had no modifier guard, so `Ctrl+Z` (the browser/OS undo chord) ALSO matched the bare `z` shortcut (add zoom) - the async add-zoom apply usually won the race against the synchronous undo, so `Ctrl+Z` visibly appeared to undo by adding a zoom instead. Shift is deliberately excluded from the guard set: `"?"` is what a US-layout browser delivers in `.key` for Shift+/, so matching it directly (see below) handles that combo without special-casing `shiftKey`.
- **`"?"` -> `"overlay"`, guarded against `repeat`.** Checked right after the modifier guard, before the lowercase dispatch below (`"?".toLowerCase()` would still be `"?"`, so order doesn't functionally matter here, but it reads top-to-bottom as "the overlay key first"). The repeat guard: `useEditorKeymap` fires `onOverlay` as a plain toggle (open<->closed) on every "overlay" action, so without it, holding `?` down would flicker `ShortcutsOverlay` open/closed at the OS key-repeat rate instead of opening it once.
- **`z`/`s` -> `"zoom"`/`"spotlight"`, case-insensitive, guarded against `repeat`.** Holding the key down must not spam regions at the OS key-repeat rate.
- **`Delete`/`Backspace` -> `"delete"` only when `hasSel`.** Repeat is ALLOWED here (unlike zoom/spotlight) - holding Delete to clear several selections in a row is a reasonable thing to do, and each keydown re-evaluates `hasSel` against whatever is selected at that moment (the hook clears `sel` after each delete, so a genuine held-key repeat naturally stops mattering once nothing is left selected).
- **`" "` (Space) -> `"play"`.** Repeat allowed (holding Space to keep playing is harmless - the hook just toggles `playing` on every keydown, so this multi-fires, which is an existing tradeoff kept as-is, not introduced by this guard pass).
- Anything else -> `null`.

### Used by

`useEditorKeymap` (`src/editor/hooks/useEditorKeymap.ts`) - the sole caller; switches on the result to run the corresponding op.
