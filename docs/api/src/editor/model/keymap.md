# src/editor/model/keymap.ts

Pure key->action decision extracted from `useEditorKeymap` so the modifier/repeat guards are unit-testable without a DOM `KeyboardEvent` or a mounted hook.

## KeyLike

```ts
export interface KeyLike { key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; repeat: boolean; shiftKey?: boolean }
```

The minimal shape `keyAction` reads off a `KeyboardEvent` - a plain interface (not the DOM type) so tests can pass object literals directly.

`shiftKey` is optional because exactly two rows read it (`Tab` and `Ctrl+1..9`, both M1a A5) and every existing literal in the tests predates it. Every other row is deliberately shift-blind: `"?"` is what a US-layout browser already delivers in `.key` for Shift+/.

## keyAction

```ts
export type KeyAction =
  "delete" | "zoom" | "spotlight" | "text" | "split" | "play" | "overlay" | "deselect" | null;

export function keyAction(e: KeyLike, hasSel: boolean): KeyAction
```

Decides which editor shortcut (if any) one keydown triggers.

### Inputs

- `e: KeyLike` - the keydown's `key`/`ctrlKey`/`metaKey`/`altKey`/`repeat`.
- `hasSel: boolean` - whether something is currently selected in the timeline; gates `"delete"`.

### Returns

`KeyAction` - `null` when nothing should fire.

### Behavior

The table, in the order the function checks it:

| Key | Action | Repeat |
|---|---|---|
| `Ctrl+Space` | `"maximize"` - blow the hovered editor area up to fill the body, or restore it (M1a) | ignored |
| `Ctrl+1` .. `Ctrl+9` | `"workspace"` - switch to that workspace on the rail (M1a A5) | allowed |
| any other Ctrl / Cmd / Alt chord | `null` | - |
| `?` | `"overlay"` - toggle `ShortcutsOverlay` | ignored |
| `Tab` (no Shift) | `"focus-properties"` - jump to the first Properties area (M1a A5) | allowed |
| `z` / `Z` | `"zoom"` | ignored |
| `s` / `S` | `"spotlight"` | ignored |
| `t` / `T` | `"text"` | ignored |
| `b` / `B` | `"split"` | ignored |
| `Delete` / `Backspace` | `"delete"`, only with a selection | allowed |
| `Escape` | `"deselect"`, only with a selection | allowed |
| `Space` | `"play"` | allowed |

- **`Ctrl+Space` -> `"maximize"` (M1a), checked BEFORE the modifier guard** since it is the one deliberate chord in the table. The match is narrow on purpose - `ctrlKey && !altKey && !metaKey` - so `Ctrl+Alt+Space` and Cmd+Space (which is the OS's own on macOS) still fall through to the guard and fire nothing. Repeat is ignored for the same reason `?` ignores it: the binding is a toggle, so holding it would flicker the area in and out at the OS key-repeat rate. `useMaximize` (`shell/frame/useMaximize.ts`) is what acts on it; `useEditorKeymap` has no case for it and ignores it.
- **`Ctrl+1..9` -> `"workspace"` (M1a A5), the second chord, checked in the same place and for the same reason.** Which workspace is `workspaceIndex` above, not the action. Repeat is ALLOWED: holding it re-picks the workspace already on screen, which costs nothing.
- **Modifier guard.** Returns `null` whenever `ctrlKey || metaKey || altKey` is true - for everything the chord above did not already claim. This is the actual bug fix: `z`/`s` previously had no modifier guard, so `Ctrl+Z` (the browser/OS undo chord) ALSO matched the bare `z` shortcut (add zoom) - the async add-zoom apply usually won the race against the synchronous undo, so `Ctrl+Z` visibly appeared to undo by adding a zoom instead. Shift is deliberately excluded from the guard set: `"?"` is what a US-layout browser delivers in `.key` for Shift+/, so matching it directly (see below) handles that combo without special-casing `shiftKey`.
- **`"?"` -> `"overlay"`, guarded against `repeat`.** Checked right after the modifier guard, before the lowercase dispatch below (`"?".toLowerCase()` would still be `"?"`, so order doesn't functionally matter here, but it reads top-to-bottom as "the overlay key first"). The repeat guard: `useEditorKeymap` fires `onOverlay` as a plain toggle (open<->closed) on every "overlay" action, so without it, holding `?` down would flicker `ShortcutsOverlay` open/closed at the OS key-repeat rate instead of opening it once.
- **`Tab` -> `"focus-properties"` (M1a A5), Shift+Tab -> `null`.** This is the one row that reads `shiftKey`, because Shift+Tab carries no distinct `.key` and has to stay the browser's own reverse traversal - the only way back out of wherever the jump landed. `useShellKeys` additionally stands down when the focus is ALREADY inside the target area, so a second press walks that area's own controls instead of bouncing off its body.
- **`z`/`s`/`t` -> `"zoom"`/`"spotlight"`/`"text"`, case-insensitive, guarded against `repeat`.** Holding the key down must not spam regions at the OS key-repeat rate. `t` (Batch 2c) adds a TITLE, the default of the four text kinds, because a single key cannot choose between four seeds and the title is the one that needs no second line; the four Add pills and the four drop types are where a kind is picked. Like the other two it is a bare letter with the modifier guard in front of it, so `Ctrl+T` stays the browser's own.
- **`b`/`B` -> `"split"`, case-insensitive, guarded against `repeat` (Batch 4 T9), the same rule as `z`/`s`/`t`.** Batch 4's spec (6.6) asked for `S` for split, but `S` has been Add spotlight since M1a shipped, so the blade took `B` instead - `ShortcutsOverlay` lists it as `B` for the same reason.
- **`Delete`/`Backspace` -> `"delete"` only when `hasSel`.** Repeat is ALLOWED here (unlike zoom/spotlight) - holding Delete to clear several selections in a row is a reasonable thing to do, and each keydown re-evaluates `hasSel` against whatever is selected at that moment (the hook clears `sel` after each delete, so a genuine held-key repeat naturally stops mattering once nothing is left selected).
- **`Escape` -> `"deselect"` only when `hasSel` (M1a).** Selection became its own axis when the areas landed - a panel tab change no longer clears it - so leaving a selection needed a key of its own. With nothing selected it returns `null` rather than a consumed no-op, which keeps Escape available to whatever surface wants it (a dialog's dismiss, arrange mode's exit). Repeat is allowed for the same reason `"delete"` allows it: there is nothing to spam. `useEditorKeymap` additionally withholds the action while `escOwned` is true.
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

Gates bare Space ONLY - `Ctrl+Space` ("maximize") is a chord no button or ARIA widget claims, so a focused `Switch` must not swallow it the way it swallows Space.

True for a native `BUTTON`/`SELECT`, or an element whose ARIA `role` is one that owns its own Space activation (`switch`, `button`, `checkbox`, `radio`, `menuitem`, `tab`, `combobox`, `option`). *Why this exists:* the global keydown listener used to `preventDefault()` every Space unconditionally before toggling play - for a native `<button>`, Chromium's own click-on-keyup simulation is entirely skipped once `preventDefault` runs on the keydown, so a focused `Switch` (`controls/fields/Switch.tsx`, a real `<button role="switch">`) or `Picker` (`controls/fields/Picker.tsx`, a real `<button>`) never got its own Space activation; playback toggled instead. `ownsSpace` only gates SPACE - it deliberately does NOT block z/s/delete/overlay for a target that owns Space, so Tab-focusing a button and pressing `z` still adds a zoom.

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

- `useEditorKeymap` (`src/editor/hooks/input/useEditorKeymap.ts`) - switches on the result to run the corresponding op. It has no `"maximize"` case and ignores that one.
- `useShellKeys` (`src/editor/shell/useShellKeys.ts`, M1a A5) - the only consumer of `"workspace"` and `"focus-properties"`, on a capture listener for the same reason `useMaximize` uses one.
- `useMaximize` (`src/editor/shell/frame/useMaximize.ts`, M1a) - the only consumer of `"maximize"`. It listens in the CAPTURE phase on `window`, so it can consume `Ctrl+Space` (and, while an area is maximized, `Escape`) before `useEditorKeymap`'s bubble-phase listener sees it. Going through `resolveKeyAction` is what gives the shell's own key the same two guarantees as every other one: inert while typing, inert behind a modal.
