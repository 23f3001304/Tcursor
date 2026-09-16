# src/editor/controls/fields/Picker.tsx

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
export function Picker<T extends string>({ value, options, onChange, ariaLabel, label }: {
  value: T; options: { value: T; label: string; title?: string; badge?: string }[];
  onChange: (v: T) => void; ariaLabel?: string; label?: string;
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
- `options[].badge?: string` (M4) - one short word rendered after the label as an `.e-picker-badge`
  chip, in BOTH the closed button (for the selected option) and every open-menu row. *Why a prop
  rather than letting the caller pass a node as `label`:* the chip has to sit inside the button's
  own flex row to keep the label's ellipsis working, and the same string has to render in two
  places the caller cannot reach. *Why it exists:* `AiPanel` badges a vision-capable engine
  "Vision" - whether a model can actually LOOK at the recording is the one thing its name cannot
  say, and it changes what a run will do. Omitted everywhere else, so no other picker changes.
- `label?: string` (Batch 2b) - overrides the text shown on the CLOSED button only. The menu, the
  `aria-selected` marks and `onChange` all still run off `value`, so the picker still reports one
  of its own options as selected and nothing about picking changes. *Why it exists:* the colour
  grade needs a "Custom" state that is NOT a value - spec 3.1 says picking a look writes three
  absolute numbers the user may then bend, and once bent the button should stop claiming to be
  that look while the menu still shows which one it started from. *Why an override rather than a
  synthetic option:* every option in the menu is a clickable row, so a "Custom" entry would be
  selectable and would have to be ignored on click, and it would sit in the list forever. *Why not
  a falsy `value`:* the component already falls back to the raw `value` string when it matches no
  option, which would render `custom` rather than `Custom` and would break `aria-selected`.
  Omitted everywhere except `GradeSection`, so no other picker changes.

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

### Where the menu opens (width/clipping audit, 2026-09-14)

The menu is **portalled** out of the picker (`portalHost`, `popoverPlace.md`) and positioned in
viewport coordinates by `placeStacked`: below the button, flipping **upward** when the window has no
room below, and clamped `EDGE_MARGIN` off every edge. Its width is the button's own `offsetWidth`,
so it still lines up with the control exactly as `left: 0; right: 0` used to.

*The bug this fixes:* `position: absolute; top: 100%` put the menu inside whatever scroll box the
picker was sitting in. Opened near the foot of an `.e-panel` it only extended that panel's scroll
height (so it was off screen until you scrolled), and inside `.e-panel-slot`, the `.e-props-side`
inspector column or a settings dialog it was simply cut off - the "Spotlight Mode" picker in
`EffectInspector` and the Theme and "Clicks to zoom" pickers in the settings dialog all could.

Three consequences worth knowing about:

- **Outside-click detection checks both boxes.** A mousedown on an option is outside the picker's
  own container now; closing on it would unmount the menu before the option's own `click` could
  fire, so the handler ignores anything inside the menu as well.
- **Scroll and resize close the menu.** A `position: fixed` layer is pinned to where the button
  *was*. The scroll listener is registered in the capture phase, because the scroll that matters is
  a panel's own and does not bubble to the window - and it exempts scrolls originating inside the
  menu, which is a scroll box itself (a long model list) and would otherwise close as it was read.
- **It is measured before it is shown,** the same way `Tooltip` is: one hidden pass, a
  `useLayoutEffect` that measures `scrollHeight` (capped at the menu's own `MENU_MAX_H`) and places
  it, then the real render. The enter/exit `y` flips with `Placement.flipped`.

### Look

The closed button and the menu are both raised planes with no stroke (34px button, `--e-raised`, one
lightness step on hover). The selected option is a **3px accent tick** at the row's left edge rather
than a filled or outlined chip, so a long menu stays quiet while it is being scanned. The menu's
paint moved out of `Picker.tsx`'s inline style and into `.e-picker-menu` (`controls/controls.css`) -
including the `z-index`, which has to clear the modals (100) now that three dialogs hold pickers;
only its placement and scroll box are still inline.
