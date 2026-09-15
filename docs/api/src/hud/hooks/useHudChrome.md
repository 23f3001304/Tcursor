# src/hud/hooks/useHudChrome.ts

Everything the HUD can have OPEN on top of the bar - a dropdown, the "what to record" sheet, the take pill's Sources sheet, and the Settings / Preferences panel - plus the rules about what closing one does to the others. Split out of `Hud.tsx` because those four flags were never independent: half the file's handlers existed only to clear three of them while setting the fourth.

## HudPanel

```ts
export type HudPanel = "settings" | "preferences" | null;
```

Which panel has the idle card's body. `null` is the card itself.

## useHudChrome

```ts
export function useHudChrome(recording: boolean): {
  menu: string | null;
  sheet: boolean;
  sources: boolean;
  panel: HudPanel;
  setSheet: (open: boolean) => void;
  setSources: Dispatch<SetStateAction<boolean>>;
  toggleMenu: (id: string) => void;
  closeMenu: () => void;
  openPanel: (p: "settings" | "preferences") => void;
  closePanel: () => void;
  picked: (run: () => void) => void;
  openSources: (clearErr: () => void) => void;
};
```

### Props

- `recording: boolean` - a take running. Only used to close everything when one ends.

### Returns

- `menu` - the id of the open dropdown, or `null`. One slot, because the idle card and the take pill's sheet are never on screen together.
- `sheet` - whether the card's body is flipped to `TargetSheet`, and likewise the Sources sheet's own display list. One flag serves both for the same reason.
- `sources` - whether the take pill's Sources sheet is open. `Hud` renders it only while `sources && !saving`: there is no take left to switch anything on once Saving starts.
- `panel` - see `HudPanel`. `Hud` builds the element for it and hands both to `IdleCard`, where `panel` is the `AnimatePresence` key.
- `toggleMenu(id)` / `closeMenu()` - open a dropdown, close the one that is open, or close whatever is open. A pick calls `closeMenu` rather than `toggleMenu`, so it cannot reopen the list it just chose from.
- `openPanel(p)` - clears `menu` and `sheet`, then sets the panel. *Why it clears the other two:* the panel takes the card's body, so an open dropdown would be gone from the screen while `menu` still held the window 140px taller for it (`MENU_OVERFLOW`), and a display picker left flipped would be what the user lands back on. No window call at all - the panel is a sheet inside the card.
- `closePanel()` - passed to both panels as their `onClose` (their own back arrow).
- `picked(run)` - runs a source switch and then closes the menu, the display list and the Sources sheet in one go, so a pick applies at once and returns the user to the take.
- `openSources(clearErr)` - toggles the Sources sheet, closes the menu and the display list, and clears the source-switch error, so a stale failure does not outlive the sheet that caused it.

### Behavior

**End of take.** An effect on `recording` clears all four the moment one ends, so the idle card never comes back mid-flip, with a stale menu hanging off it, or on a panel nobody asked for.

**Escape.** A `keydown` listener on `window`, mounted only while `panel` is set, closes it. It ignores an event whose `defaultPrevented` is already set, so the hotkey-capture Escape inside Settings (`SettingsHotkeys` calls `preventDefault` on every key it captures) cancels the capture without also closing the panel around it.

### Used by

- `src/hud/Hud.tsx`.
