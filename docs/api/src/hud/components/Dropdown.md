# src/hud/components/Dropdown.tsx

Controlled custom select-menu component used by the HUD bar for camera, display, and microphone device selection. Because the menu list overflows the bar's native window bounds, the HUD resizes the window taller while any dropdown is open. This file exports one interface and one component.

## DropOption

```ts
export interface DropOption { id: string; label: string }
```

Shape of a single selectable item.

- `id: string` - opaque identifier passed back to the parent via `onPick`. *Why opaque:* callers (Hud) map device IDs from the OS; the Dropdown needs no knowledge of their format.
- `label: string` - human-readable string rendered in the trigger button and in the list items.

### Used by

- `src/hud/components/IdleCard.tsx` - the camera and mic rows (`row`), fed `DropOption[]` arrays `Hud` builds from `cameras` and `mics`. The display/window target is not a dropdown at all: the screen row flips the card to `TargetSheet` (`src/hud/devices/TargetSheet.md`), since two monitors are told apart by a map, not a list.

## Dropdown

```ts
export function Dropdown({ icon, value, options, open, onToggle, onPick }: {
  icon?: ReactNode;
  value: string;
  options: DropOption[];
  open: boolean;
  onToggle: () => void;
  onPick: (id: string) => void;
}): JSX.Element
```

Renders a trigger button and, when `open` is true, a floating option list below it. The currently selected option is highlighted with `.sel` and a `<Check>` icon.

### Props

- `icon` (`ReactNode`, optional) - icon displayed to the left of the label in the trigger. *Why optional:* a bare text-only trigger is valid; the camera, display, and mic dropdowns each supply their own icon.
- `value` (`string`) - id of the currently selected option. The trigger resolves its display label by searching `options`; falls back to `options[0].label` if no match is found, and to `"--"` if `options` is empty. *Why controlled:* keeps all device state in `Hud` with no duplication.
- `options` (`DropOption[]`) - the full list of selectable items rendered in the menu. *Why passed in full:* Dropdown has no enumeration logic; that lives in `useDevices` and `useCameraDevices`.
- `open` (`boolean`) - controlled open/closed state driven by the parent's `menu` string. *Why controlled:* `Hud` needs to know which dropdown is open to compute the window height.
- `onToggle` (`() => void`) - called when the trigger button is clicked, or when an outside click is detected while open. *Why a single callback:* the parent manages the `menu` toggle; Dropdown just signals intent.
- `onPick` (`(id: string) => void`) - called with the chosen option's `id` when the user clicks a list item. *Why only the id:* the label is derivable from the id; passing just the key keeps the callback narrow.
- `row?: boolean` - render the trigger as the idle card's full-width source row (`.dd-row`: a raised 46px plane with a 34px icon lead) instead of the compact ghost `.dd-trigger`. The menu then opens full width under the row.

### Behavior

An outside-click listener is attached to `document` via a `useEffect` on `[open, onToggle]`. When `open` is true, any `mousedown` event whose target is outside the root `ref` div calls `onToggle` to close the menu. The listener is removed when `open` becomes false or on cleanup. *Why `mousedown` rather than `click`:* mousedown fires before the click event, preventing brief re-open races when clicking the trigger of another dropdown.

The menu list is a `motion.div` (design/premium-pass D6, replacing a CSS `@keyframes` animation 1:1) that fades/slides in on mount (opacity + `y: -4 -> 0`, 0.14s). Deliberately enter-only, no `AnimatePresence`: closing is still a hard cut, same as before D6 - `Hud.tsx`'s `useHudWindowSize` shrinks the actual OS window back to its idle height the instant the menu closes (synchronously with the same `menu` state that gates `open`), so an animated exit here would visibly get clipped by the window shrinking underneath it rather than fading cleanly. The trigger itself gets the app-wide press spring (scale .96). Each list item is a `<button>` so it is keyboard-focusable; the selected item additionally renders a `<Check>` icon.

### Notes

- The component holds no local open state. All state lives in `Hud` via the `menu` string.
- Fallback label resolution (`options.find(...) ?? options[0]?.label ?? "--"`) means an empty `options` array never throws; it renders "--" in the trigger.
