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
  value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; ariaLabel?: string;
}): JSX.Element
```

### Props

- `ariaLabel?: string` - (Task 26) sets `aria-label` on the toggle button and the `role="listbox"`
  menu. Every call site in the codebase passes this.

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
