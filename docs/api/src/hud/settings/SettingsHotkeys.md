# src/hud/settings/SettingsHotkeys.tsx

Settings panel for keyboard chord bindings. Provides inline capture of new chords and real-time conflict detection. Exports two utility functions (`chordFromEvent`, `conflicts`) that are independently testable and reusable outside the component.

## chordFromEvent

```ts
export function chordFromEvent(e: KeyboardEvent): string | null
```

Converts a native `KeyboardEvent` into a chord string (e.g., `"Ctrl+Alt+Z"`), or returns `null` if the event should be ignored.

### Inputs

- `e: KeyboardEvent` - the raw DOM keyboard event from a `keydown` listener.

### Returns

A string of the form `"[Ctrl+][Alt+][Shift+]KEY"` where KEY is one alphanumeric character A-Z or 0-9, or `null` in two cases:
- `e.key` (uppercased) is a modifier key itself (Control, Alt, Shift, Meta). *Why:* pressing only modifiers is not a complete chord; the user is still building one.
- `e.key` does not match `/^[A-Z0-9]$/`. *Why:* restricting to letters and digits mirrors the Rust `KeyChord` format and avoids platform differences in how special keys are named.

Modifier prefix order is always Ctrl, Alt, Shift - matching the Rust backend's `KeyChord` serialization format so JS-captured chords can be compared directly to Rust-parsed ones.

## conflicts

```ts
export function conflicts(h: HotkeySettings): Set<string>
```

Returns the set of chord strings bound to more than one action.

### Inputs

- `h: HotkeySettings` - the full hotkey settings object (all eight bindings).

### Returns

`Set<string>` of chord strings that appear in more than one `HotkeySettings` field. Empty when no conflicts exist. Uses an internal `seen: Map<string, number>` to count occurrences across the eight `ROWS` entries, then filters to those with count > 1.

*Why a `Set` of chord strings rather than action keys:* the caller needs to highlight every `.hk-chord` button whose value matches a conflicting chord, which is a string lookup - not an action-key lookup.

## SettingsHotkeys

```tsx
export function SettingsHotkeys({
  value,
  onChange,
}: {
  value: HotkeySettings;
  onChange: (v: HotkeySettings) => void;
})
```

Edits the `hotkeys` group of `Settings`.

### Props

- `value: HotkeySettings` - current chord bindings for all eight actions. *Why:* controlled component; the parent (Preferences) persists state.
- `onChange: (v: HotkeySettings) => void` - receives a fully replaced `HotkeySettings` when a binding is committed. *Why:* immutable replacement keeps persistence uniform with all other panels.

### Behavior

Renders `<section className="sec">` with heading "Hotkeys". Holds one piece of local state: `capturing: keyof HotkeySettings | null` - the action currently awaiting a new chord, or null when no capture is in progress.

Eight rows from the internal `ROWS` constant are rendered in order:

| Label | Field |
|---|---|
| Hold to zoom | `zoom_hold` |
| Spotlight hold | `spotlight_hold` |
| Video effect hold | `video_fx_hold` |
| Layout: Screen | `layout_screen` |
| Layout: Camera | `layout_camera` |
| Layout: Presenter | `layout_presenter` |
| Layout: Screen only | `layout_screen_only` |
| Layout: Camera only | `layout_camera_only` |

Each row renders a `.sf-row.hk` with a `.sf-label` and a `.hk-chord` button:

- When `capturing !== key` - the button shows the current chord string (e.g., `"Ctrl+Z"`) and clicking it sets `capturing = key`.
- When `capturing === key` - the button shows `"press keys..."` and listens on `onKeyDown`. A valid chord from `chordFromEvent` commits `onChange({ ...value, [key]: chord })` and resets `capturing` to null. `Escape` cancels without writing.

Conflict state is computed from `conflicts(value)` on every render:
- Any `.hk-chord` button whose `value[key]` is in the conflict set receives the `.dup` class (typically highlighted red).
- When `dup.size > 0`, a `.hk-warn` paragraph "Two actions share a shortcut." appears below all rows.

### Notes

- Chord capture uses `e.preventDefault()` to suppress browser shortcuts while the panel is listening.
- `ROWS` is a file-local constant array; it determines both display order and which `HotkeySettings` fields are editable. Adding a new binding requires updating `ROWS`, `HotkeySettings` (in `settings.ts`), and the Rust backend's chord map.
