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

`resolveKeyAction` (below), the sole caller of `keyAction` now that it folds in the DOM/modal context too.

## TargetLike

```ts
export interface TargetLike { tagName: string; role: string | null; isContentEditable: boolean }
```

The minimal shape `useEditorKeymap` needs off `document.activeElement` - kept as a plain interface (not the DOM type) for the same testability reason as `KeyLike`. Used by `isTypingTarget`, `ownsSpace`, and `resolveKeyAction` below (all added bug-sweep-2 Task 8, M4).

## isTypingTarget

```ts
export function isTypingTarget(t: TargetLike): boolean
```

True for `INPUT`/`TEXTAREA`/`isContentEditable`. The ORIGINAL "don't fire shortcuts while typing" guard, applies to every action equally (typing `s`/`z` into a text field must never add a spotlight/zoom) - now expressed as a pure predicate instead of inline in the hook.

## ownsSpace

```ts
export function ownsSpace(t: TargetLike): boolean
```

True for a native `BUTTON`/`SELECT`, or an element whose ARIA `role` is one that owns its own Space activation (`switch`, `button`, `checkbox`, `radio`, `menuitem`, `tab`, `combobox`, `option`). *Why this exists:* the global keydown listener used to `preventDefault()` every Space unconditionally before toggling play - for a native `<button>`, Chromium's own click-on-keyup simulation is entirely skipped once `preventDefault` runs on the keydown, so a focused `Switch` (`controls/Switch.tsx`, a real `<button role="switch">`) or `Picker` (`controls/Picker.tsx`, a real `<button>`) never got its own Space activation; playback toggled instead. `ownsSpace` only gates SPACE - it deliberately does NOT block z/s/delete/overlay for a target that owns Space, so Tab-focusing a button and pressing `z` still adds a zoom.

**`role="slider"` is deliberately NOT in the set (review round 1 minor).** Per WAI-ARIA authoring practices a slider does not own Space (only arrow keys move it), and `Slider.tsx` doesn't handle Space either - including it here made Space a DEAD key whenever a `Slider` had focus: `resolveKeyAction` withheld `"play"` (since `ownsSpace` was true), but nothing else consumed the keydown, so neither the global toggle NOR the slider itself reacted to it. Removed, so Space on a focused Slider now correctly falls through to the normal `"play"` toggle.

## resolveKeyAction

```ts
export function resolveKeyAction(e: KeyLike, ctx: { hasSel: boolean; modalOpen: boolean; shortcutsOpen: boolean; target: TargetLike }): KeyAction
```

The FULL decision `useEditorKeymap` acts on (bug-sweep-2 Task 8, M4) - folds `keyAction`'s key mapping together with the DOM/modal context it alone can't see. Kept as a separate function (not a wider `keyAction` signature) so `keyAction`'s own key-mapping tests stay untouched.

- `isTypingTarget(ctx.target)` is checked FIRST and unconditionally blocks every action, `?` included - typing a literal `?` into a text field must never toggle the overlay.
- **`?` is special-cased AHEAD of the general modal bail (review round 1 minor).** `e.key === "?"` returns `null` only when `ctx.modalOpen && !ctx.shortcutsOpen` (blocked behind some OTHER modal - must not pop the overlay open on top of, say, the Export dialog); otherwise it falls through to `keyAction(e, ctx.hasSel)` regardless of `ctx.modalOpen`. *Why:* `?` is the toggle that OPENS `ShortcutsOverlay`, and `useEditorKeymap`'s `onOverlay` wiring is a toggle, not a one-way open - so once the overlay IS the open modal (`ctx.shortcutsOpen`), `modalOpen` is ALSO true (it's one of the ORs that make it true), and without this special case `?` would be unable to close what it opened, even though Escape/a scrim click still can.
- For every OTHER key, returns `null` immediately when `ctx.modalOpen` (a dialog/overlay/confirm/the AI director's own scrim sits on top - Export/Settings/Shortcuts/`ConfirmDialog`/`running`) - every shortcut is inert in that case.
- Otherwise calls `keyAction(e, ctx.hasSel)`; if the result is `"play"` AND `ownsSpace(ctx.target)`, returns `null` instead (see `ownsSpace` above) - every other action passes through unchanged.

### Used by

`useEditorKeymap` (`src/editor/hooks/useEditorKeymap.ts`) - the sole caller; switches on the result to run the corresponding op.
