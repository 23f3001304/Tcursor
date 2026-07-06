# src/hud/settings/SettingsPanel.tsx

Settings overlay panel with four tabs (Zoom, Cursor, Keys, FX) for configuring recording behavior. Loads the full persisted settings on mount into a local draft and writes every change back via IPC immediately. Opened from the Hud titlebar via `openPanel("settings")`.

## Settings

```ts
export function Settings({ onClose }: { onClose: () => void }): JSX.Element | null
```

A segmented-tab settings box. Returns `null` until the initial `getSettings()` resolves (prevents a flash of empty UI during async load).

### Props

- `onClose` (`() => void`) - called when the back-arrow button in the panel header is clicked. *Why a callback:* `Hud` owns the panel state and the window morph; it drives the transition back to the bar via `setPanel(null)`, which triggers `AnimatePresence`'s exit animation and then `restoreBar`.

### Behavior

**State:**
- `draft` (`Settings | null`) - the full settings object. The component renders `null` until `getSettings()` resolves. *Why a local draft:* optimistic updates keep the UI snappy; `patch` applies locally and writes to disk simultaneously.
- `tab` (`"zoom" | "cursor" | "keys" | "fx"`) - the currently active tab, initialized to `"zoom"`.

**Mount effect (`useEffect` on `[]`):**
Calls `getSettings()` and sets `draft` on success. Errors are swallowed with `.catch(() => {})`.

**`patch(next: S)`:**
Sets `draft` to `next` and calls `setSettings(next)` (fire-and-forget, errors silently ignored). *Why no debounce:* settings writes are cheap and debouncing risks losing the last value before the panel closes.

**Tab animation:**
Tab panels are hosted inside `AnimatePresence mode="wait"`. Each `motion.div` (keyed by `tab`) animates in from `{ opacity: 0, x: 8 }` and exits to `{ opacity: 0, x: -8 }` over 160ms with ease `[0.4, 0, 0.2, 1]`. *Why `mode="wait"`:* ensures the exiting panel disappears before the entering one starts.

**Tab content:**

| id | Label | Component rendered |
|---|---|---|
| `"zoom"` | Zoom | `<SettingsZoom>` for zoom config plus a `<Range>` for mic sync offset (`audio_offset_ms`, -300 to +300 ms, step 10) |
| `"cursor"` | Cursor | `<SettingsCursor>` |
| `"keys"` | Keys | `<SettingsHotkeys>` |
| `"fx"` | FX | `<SettingsClickFx>` |

The Zoom tab uniquely stacks two children: `<SettingsZoom>` and a mic-sync offset `<Range>` labelled "Mic sync offset" with hint "- earlier - + later", formatted as `+N ms` / `N ms`. *Why on the Zoom tab:* audio offset is conceptually paired with zoom timing since both affect the rendered export's temporal alignment.

**Panel header:**
Contains a `<Back />` button wired to `onClose` and the title "Settings". The header div uses `data-tauri-drag-region` so the window remains draggable while the bar is hidden.

### Notes

- Tab definitions are a module-level constant `TABS: { id: Tab; label: string }[]` with four entries; they drive both the tab button row and the conditional render block.
- The `Range` for mic sync offset rounds to integer milliseconds via `Math.round(v)` because the underlying `audio_offset_ms` field is typed as a number but represents a whole-millisecond offset.
