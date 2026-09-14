# src/hud/preferences/Preferences.tsx

Preferences panel with "Interface" and "Layout" tabs for theme/accent and recording-layout appearance settings. Loads the full settings object from Tauri on mount, holds it as a local draft, and writes every change back via IPC immediately with no debounce. Opened from the idle card's header via `openPanel("preferences")`, and rendered as a sheet inside that card's body (`IdleCard`'s `panelBody`) - the same flip, and the same 430px `--sheet-h` box, the display picker gets, so the window never resizes for it. A theme or accent change applies to the card around it immediately (`onUiChange` -> `applyTheme`), which is now visible while the panel is still open.

## Preferences

```ts
export function Preferences({ onClose, onUiChange }: {
  onClose: () => void;
  onUiChange: (ui: InterfaceSettings) => void;
}): JSX.Element | null
```

A two-tab settings panel. Returns `null` until the initial `getSettings()` resolves (prevents a flash of empty UI during async load).

### Props

- `onClose` (`() => void`) - called when the back-arrow button in the panel header is clicked. *Why a callback rather than internal navigation:* `Hud` owns the panel state; `setPanel(null)` re-keys the card's flip, which frosts this panel out and the sources back in. `Hud`'s Escape listener calls the same thing.
- `onUiChange` (`(ui: InterfaceSettings) => void`) - called with the FULL new `InterfaceSettings` whenever the Interface tab changes anything (theme, accent, or Task 39's `animated_brand`). *Why propagated up:* `Hud` holds `themeRef` and must call `applyTheme` immediately so CSS variables update in the current session without waiting for the next settings reload, and (Task 39) sets its own `animatedBrand` state so the titlebar mark reacts immediately too. *Why the whole object rather than positional args (renamed from `onThemeChange`, Task 39):* a growing list of individual changed-field parameters doesn't scale past two: passing the full `InterfaceSettings` once covers every current AND future field this tab writes, with one call site to update instead of one per field.

### Behavior

**State:**
- `draft` (`Settings | null`) - the full settings object. The component renders `null` until the `getSettings()` promise resolves. *Why a local draft:* allows optimistic local updates; every `patch(next)` call updates the UI immediately and also fires `setSettings(next)` (fire-and-forget, errors silently ignored).
- `tab` (`"interface" | "layout"`) - the currently active tab, initialized to `"interface"`.

**Mount effect (`useEffect` on `[]`):**
Calls `getSettings()` and sets `draft` on success. Errors are swallowed with `.catch(() => {})`. *Why no retry:* a failure here means Tauri is unreachable, which is a fatal condition the broader app would surface separately.

**`patch(next: S)`:**
Sets `draft` to `next` and calls `setSettings(next)`. Errors are silently ignored. *Why no debounce:* settings writes are cheap (a JSON file write on the Rust side) and debouncing would mean the last slider value before closing might not persist.

**Tab animation:**
Tab panels are wrapped in `AnimatePresence mode="wait"`. Each `motion.div` (keyed by `tab`) slides in from `{ opacity: 0, x: 8 }` and exits to `{ opacity: 0, x: -8 }` over 160ms with ease `[0.4, 0, 0.2, 1]`. *Why `mode="wait"`:* ensures the exiting tab fully disappears before the entering one starts, preventing overlap.

**Tab content:**
- `"interface"` tab: renders `<SettingsInterface>` receiving `draft.ui`. Its `onChange` calls `patch({ ...draft, ui })` and additionally calls `onUiChange(ui)` to propagate the change to `Hud`.
- `"layout"` tab: renders `<SettingsAppearance>` receiving `draft.appearance`. Its `onChange` calls `patch({ ...draft, appearance })`.

### Notes

- The panel header (a 30px row matching the display picker's `.sheet-head`) keeps `data-tauri-drag-region`, so a drag on it still moves the window, the same as the card header above it.
- Tab definitions are a module-level constant `TABS: { id: Tab; label: string }[]` with `"interface"` and `"layout"` entries; they drive both the tab button row and the conditional rendering block.
