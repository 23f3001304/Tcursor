# src/editor/panels/CursorPanel.tsx

Editor panel for the "Cursor" rail tab. Toggles synthetic-cursor visibility and click-bounce, lists/selects the cursor sprite pack (built-in + any imported), offers a folder-based "Import pack..." action via the Tauri dialog plugin, and holds the size/motion-blur/bounce-intensity sliders.

## CursorPanel

```tsx
export function CursorPanel({ settings, onChange, onClose }: { settings: CursorSettings; onChange: (v: CursorSettings) => void; onClose: () => void }): JSX.Element
```

Renders the full Cursor settings panel.

### Props

- `settings: CursorSettings` - the current per-project cursor settings (`doc.settings.cursor`). *Why per-project, not global:* pack/size/style are edited post-record, alongside every other panel, and persist into `edit.json`.
- `onChange: (v: CursorSettings) => void` - called with the full next `CursorSettings` on every control change. *Why full object, not a patch:* mirrors every other panel's `set()` helper (`{ ...settings, [k]: v }`), keeping `Editor.tsx`'s `saveDocSettings({ ...doc.settings, cursor })` wiring uniform across panels.
- `onClose: () => void` - closes the panel (back to the AI tab).

### Behavior

**Pack list.** On mount, calls `listCursorPacks()` and stores the result in local `packs` state - the built-in "Default" pack first, then any previously imported packs. Each is rendered as an `.e-pack-card` button showing its `name`; clicking one calls `set("pack", p.id)`, which round-trips through `onChange` into `doc.settings.cursor.pack` and is persisted immediately (same save path as every other field). The selected card gets the `on` class via `settings.pack === p.id`.

**Import.** The "Import pack..." button (`.e-upload-dashed`, `IconFolderPlus`) opens the Tauri dialog plugin's folder picker (`{ directory: true, multiple: false }`). On a chosen folder, calls `importCursorPack(dir)`:
- On success: appends the returned `CursorPackInfo` to local `packs` (so it appears in the grid without a re-fetch) and immediately selects it (`set("pack", info.id)`).
- On failure: shows the rejection message (or a generic fallback if it isn't a string) as inline red text below the grid.
- While in flight, the button is disabled and its label switches to "Importing...".

**Show synthetic cursor.** A `Switch` bound to `settings.style !== "hidden"`; toggling writes `"enhanced"` or `"hidden"` to `style` (there is no separate UI for the third style, `"system"`, in this panel).

**Reset.** Restores every field to the app defaults, including `pack: "default"` - does *not* touch the locally fetched `packs` list (an imported pack stays visible in the grid after a reset; only the *selection* reverts to Default).

**Sliders.** Cursor Size (`0.4`-`3.0`), Motion Trail Blur (`0`-`1`), Click Bounce Intensity (`0.1`-`1`) - each a direct `set(field, v)` on `Slider`'s `onChange`.

### Notes

- Selecting or importing a pack takes effect in both the export and the live editor preview: `Editor.tsx` passes `doc.settings.cursor` straight through to `Stage`, and `useEditorData`'s `cursorSprites` fetch is keyed on `doc?.settings.cursor.pack` so it re-decodes sprites exactly when the pack changes (not on unrelated edits).
- No client-side validation of the picked folder happens here - `importCursorPack` (Rust) does all validation (recognized filenames, real PNG bytes, `hotspots.json` shape) and this panel just surfaces whatever error string comes back.

### Used by

- `src/editor/Editor.tsx` - rendered for the `"cursor"` rail tab, wired to `doc.settings.cursor` / `saveDocSettings`
