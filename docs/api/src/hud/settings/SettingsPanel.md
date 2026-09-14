# src/hud/settings/SettingsPanel.tsx

Settings panel with four tabs (Zoom, Cursor, Keys, FX) for configuring recording behavior. Loads the full persisted settings on mount into a local draft and writes every change back via IPC immediately. Opened from the idle card's header via `openPanel("settings")`, and rendered as a sheet inside that card's body (`IdleCard`'s `panelBody`) - the same flip, and the same box, the display picker gets. The window does not resize for it: `.settings` (`settings.css`) is pinned to the card body's `--sheet-h` (430px) and `.settings-body` scrolls inside that.

## Settings

```ts
export function Settings({ onClose }: { onClose: () => void }): JSX.Element | null
```

A segmented-tab settings sheet. Returns `null` until the initial `getSettings()` resolves (prevents a flash of empty UI during async load).

### Props

- `onClose` (`() => void`) - called when the back-arrow button in the panel header is clicked. *Why a callback:* `Hud` owns the panel state; `setPanel(null)` re-keys the card's flip, which frosts this panel out and the sources back in. `Hud`'s Escape listener calls the same thing.

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
Contains a `<Back />` button wired to `onClose` and the title "Settings". A 30px row that matches the display picker's `.sheet-head`, back arrow pulled out to the card's own left edge and no hairline under it - the tabs already separate it from the body. It keeps `data-tauri-drag-region` so a drag anywhere on it still moves the window, the same as the card header above it.

**Escape:** `SettingsHotkeys` calls `preventDefault()` on every key it captures, including its own cancel-capture Escape, and `Hud`'s Escape listener skips an event that is already `defaultPrevented` - so Escape inside a capturing chord button cancels the capture and leaves the panel open. Anywhere else in the panel, Escape closes it.

### Notes

- Tab definitions are a module-level constant `TABS: { id: Tab; label: string }[]` with four entries; they drive both the tab button row and the conditional render block.
- The `Range` for mic sync offset rounds to integer milliseconds via `Math.round(v)` because the underlying `audio_offset_ms` field is typed as a number but represents a whole-millisecond offset.
