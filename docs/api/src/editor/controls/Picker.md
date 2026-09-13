# src/editor/controls/Picker.tsx

A custom dropdown select: a toggle button + an animated (Motion) options menu, and (as of Task
26) full keyboard support - the button stays focused throughout, so keyboard users never lose
their place (WAI-ARIA describes this as a `aria-activedescendant`-based listbox).

## pickerNextIndex

```ts
export function pickerNextIndex(key: string, activeIndex: number, length: number): number | null
```

The option index `ArrowUp`/`ArrowDown` should move the active (keyboard-highlighted, not yet
committed) selection to, or `null` for any other key. From "nothing active yet"
(`activeIndex < 0`, i.e. the menu was just opened), `ArrowDown` starts at index `0` and `ArrowUp`
at the last index - the usual "open a listbox with the appropriate end active" convention.
Otherwise steps by one and clamps at `[0, length-1]` (does not wrap).

## Picker

```tsx
export function Picker<T extends string>({ value, options, onChange, ariaLabel }: {
  value: T; options: { value: T; label: string; title?: string }[]; onChange: (v: T) => void; ariaLabel?: string;
}): JSX.Element
```

### Props

- `ariaLabel?: string` - (Task 26) sets `aria-label` on the toggle button and the `role="listbox"`
  menu. Every call site in the codebase passes this.
- `options[].title?: string` (Task 11) - optional per-option hover text, set as the `title` attribute
  on both the closed button (for the currently-selected option) and each open-menu option button.
  *Why:* `AiPanel`'s Engine picker shows a short derived `label` (`engineDisplayName`) but still
  wants the full raw model id reachable on hover - `title` carries it without shortening `label`
  itself. Every other existing call site simply omits it (`title` is `undefined`), unaffected.

### Behavior (Task 26 additions)

The toggle button carries `aria-haspopup="listbox"`, `aria-expanded`, and
`aria-activedescendant` (pointing at the keyboard-active option's `id` while open). Its
`onKeyDown`: `ArrowUp`/`ArrowDown` open the menu (via `pickerNextIndex`) or move the active
option; `Enter`/`Space` open the menu, or commit the active option and close; `Escape` closes.
The menu is `role="listbox"`; each option button is `role="option"` with `aria-selected`
reflecting `value === opt.value`, plus a `--e-focus` outline on whichever option is currently
keyboard-active. Because focus never leaves the toggle button during keyboard use, "focus
returns to the button" on close is automatic - `closeMenu` also calls `buttonRef.current?.focus()`
for the mouse-interaction edge case (an option was reached via Tab, then `Escape` pressed there).

### When to use it (panel pass, 2026-09-13)

`Picker` is now for LONG lists only: the AI engine list, ripple style (seven), spotlight mode (six),
video FX mode (four, but two words each), and the inspectors' easing curves. Two to four exclusive
states with one-word labels use `Segmented` instead (`Segmented.md`), which has the same
`value`/`options`/`onChange` shape - the benchmark's section (b) point 2, "segmented controls for
exclusive states, never dropdowns".

### Look

The closed button and the menu are both raised planes with no stroke (34px button, `--e-raised`, one
lightness step on hover). The selected option is a **3px accent tick** at the row's left edge rather
than a filled or outlined chip, so a long menu stays quiet while it is being scanned. The menu's
paint moved out of `Picker.tsx`'s inline style and into `.e-picker-menu` (`controls/controls.css`);
only its placement and scroll box are still inline.
