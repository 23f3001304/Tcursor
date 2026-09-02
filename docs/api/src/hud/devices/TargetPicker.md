# src/hud/devices/TargetPicker.tsx

Capture-target picker used by the HUD bar in place of a plain `Dropdown` for the display/window selector. Same controlled open/close contract as `Dropdown` (see `src/hud/components/Dropdown.md`), but each row is decorated with its resolution and a "Primary" badge, and `kind: "window"` targets are grouped under their own "Windows" header, separate from displays. Purely presentational: the `target_id` values (`display:N` / `window:0x…`) and the selection callback are unchanged from the dropdown this replaces.

## TargetPicker

```tsx
export function TargetPicker({ targets, value, open, onToggle, onPick }: {
  targets: DisplayInfo[];
  value: string;
  open: boolean;
  onToggle: () => void;
  onPick: (id: string) => void;
}): JSX.Element
```

Renders a trigger button (Monitor icon + the selected target's cleaned title) and, when `open` is true, a floating list grouped into displays first, then a "Windows" divider and any window targets.

### Props

- `targets: DisplayInfo[]` - the full `listDisplays()` result, in backend enumeration order (displays first, then windows). *Why order matters:* `parseTarget`'s `primary` flag is keyed off raw index 0, which is only ever a display because the backend always enumerates the main monitor first.
- `value: string` - the currently selected target's `id` (a `target_id` string, e.g. `display:0` or `window:0x1a2b`). Resolved to a title via the same `parseTarget` pass used for the rows. *Why controlled:* keeps target state in `Hud`, matching `Dropdown`'s contract.
- `open: boolean` - controlled open/closed state driven by the parent's `menu` string.
- `onToggle: () => void` - called when the trigger is clicked, or on an outside click while open.
- `onPick: (id: string) => void` - called with the chosen target's raw `id` (never the parsed title) when a row is clicked. *Why the raw id:* `Hud` writes it straight into `sel.displayId`, which `startRecording`'s `targetId` argument consumes unchanged.

### Behavior

**Self-filtering.** Rows are computed as `targets.map((t, i) => ({ t, meta: parseTarget(t, i) })).filter((r) => !isOwnProcessWindow(r.t))` (`src/hud/devices/selectDevices.ts`) - `parseTarget` runs first against the raw (unfiltered) index so `primary` stays correct, then this process's own phantom window (see `isOwnProcessWindow`) is dropped from what actually renders. The app never lists itself as a capture target.

**Grouping.** The filtered rows are split into `screens` (`kind !== "window"`) and `windows` (`kind === "window"`), then rendered as `screens` followed by a `.tp-divider` header and `windows` - only when `windows.length > 0`.

**Row decoration.** Each row calls `parseTarget(t, index)` (`src/hud/devices/selectDevices.ts`) to split the backend's baked-in label into a clean `title`, an optional `resolution` sub-line, and a `primary` flag. A row renders `title` + `resolution` (if present) on the left (`.tp-main`), and a "Primary" badge (if `primary`) plus the selected-row `<Check>` on the right (`.tp-trail`).

**Outside-click close.** Identical to `Dropdown`: a `mousedown` listener on `document`, attached only while `open`, closes the menu when the click lands outside the root `ref` div.

### Notes

- Holds no local open state - identical contract to `Dropdown`, so `Hud`'s existing `menu`/`tg()` single-open-dropdown pattern works unchanged.
- The trigger's icon is always `<Monitor>`, even when the current selection is a window - matching the plain dropdown's prior behavior (no per-kind icon).
- Styled in `src/hud/hud.css` under the `tp-*` classes, layered on top of the shared `.dd`/`.dd-trigger`/`.dd-menu`/`.dd-item` rules `Dropdown` also uses, so both pickers stay visually consistent.
- Motion is identical to `Dropdown` too (design/premium-pass D6): the trigger gets the app-wide press spring (scale .96), and the menu fades/slides in on mount (opacity + `y: -4 -> 0`, 0.14s) - enter-only, same reason as `Dropdown` (see its doc): an animated exit would race `useHudWindowSize` shrinking the OS window underneath it.

### Used by

- `src/hud/Hud.tsx` - renders one `<TargetPicker>` bound to `displays`/`sel.displayId`, replacing the former display `<Dropdown>`.
