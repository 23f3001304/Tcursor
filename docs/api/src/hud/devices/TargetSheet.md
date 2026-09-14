# src/hud/devices/TargetSheet.tsx

The "what to record" sheet the idle card flips to from its Screen row: one list in two sections, every display as a row and every window as a row underneath. A dropdown list of displays and windows had nowhere to open inside a vertical card (2026-09-14); a map of the monitors drawn to scale, with screen thumbnails, was tried next and vetoed on sight (owner: "remove the thumbnail style, just make it a list"). Replaces `TargetPicker.tsx` (the dropdown version).

## TargetSheet

```ts
export function TargetSheet({ targets, value, onPick, onBack }: {
  targets: DisplayInfo[]; value: string; onPick: (id: string) => void; onBack: () => void;
}): JSX.Element
```

### Props

- `targets` - the raw `listDisplays()` result. Indexed before filtering, since `parseTarget`'s index-0 primary rule is keyed off the raw list; this process's own phantom window (`isOwnProcessWindow`) is then dropped.
- `value` - the chosen target id.
- `onPick` - one tap on a row picks it; the card closes the sheet in the same handler.
- `onBack` - the header's back arrow, closing without a pick.

### Behavior

- Header: back arrow plus "What to record". Then one scrolling `role="listbox"` with a "Displays" label, a row per display, a "Windows" label (only when any exist) and a row per window.
- A display row: the monitor glyph, the display's name (the `Display N: ` prefix dropped), and a sub-line with its size as `W by H` and `Primary` where `parseTarget` says so, joined by a middle dot. A window row: the `AppWindow` glyph and the window title with the backend's `App: ` prefix dropped. The chosen row carries a check.
- Rows are `.dd-item.tgt`: the menu's own row shape with the sub-line (`hud.css`), so the sheet and the two dropdown menus read as one family.

### Used by

- `src/hud/components/IdleCard.tsx` - the flipped card body.
