# src/editor/panels/CursorPanel.tsx

Editor panel for the "Cursor" rail tab. Picks the cursor style (System / Enhanced / Hidden); for Enhanced it also toggles click-bounce, lists/selects the cursor sprite pack (built-in + any imported), offers a folder-based "Import pack..." action via the Tauri dialog plugin, and holds the size/smoothness/path-idealization/motion-blur/bounce-intensity sliders. Every Enhanced-only control is hidden for the System and Hidden styles.

## CursorPanel

```tsx
export function CursorPanel({ settings, onChange, onClose, osCursorInVideo = true }: { settings: CursorSettings; onChange: (v: CursorSettings) => void; onClose: () => void; osCursorInVideo?: boolean }): JSX.Element
```

Renders the full Cursor settings panel.

### Props

- `settings: CursorSettings` - the current per-project cursor settings (`doc.settings.cursor`). *Why per-project, not global:* pack/size/style are edited post-record, alongside every other panel, and persist into `edit.json`.
- `onChange: (v: CursorSettings) => void` - called with the full next `CursorSettings` on every control change. *Why full object, not a patch:* mirrors every other panel's `set()` helper (`{ ...settings, [k]: v }`), keeping `Editor.tsx`'s `saveDocSettings({ ...doc.settings, cursor })` wiring uniform across panels.
- `onClose: () => void` - closes the panel (back to the AI tab).
- `osCursorInVideo?: boolean` - whether this recording's video already has the OS cursor baked in. When `false` and the style is `System`, the panel shows a one-line hint (wrapped in `AnimatePresence`, design/premium-pass D6 - fades in/out as the style picker changes, 0.14s opacity/y-4) that the cursor is re-created from the recorded path. *Why a hint and not a disabled option:* the fallback genuinely works (the renderer draws a plain arrow along the raw path), so `System` is a valid choice here - it just isn't the *original* cursor, and saying so avoids the "why does my cursor look different?" follow-up. *Why optional, defaulting to `true`:* the panel keeps rendering standalone - and in its own tests - exactly as before, with no hint.

### Behavior

**Pack list (loading/empty - Task 26).** `packs: CursorPackInfo[] | null` - `null` means the `listCursorPacks()` fetch is in flight (`loadPacks`, a `useCallback` run on mount); it resolves to the real list (built-in "Default" pack first, then any previously imported packs) or, on rejection, `[]`. Three renders of the pack grid, keyed on `packs`:
- `null` -> `PACK_SKELETON_COUNT` (5) `Shimmer` tiles (`.e-pack-card` sizing) filling one grid row.
- `[]` (resolved empty, or the fetch rejected) -> a quiet `.e-lede` line: "No cursor packs yet — Import a pack folder" (Task 26 fix - the import flow opens a folder picker, not a zip, so the copy now matches), pointing at the Import button below rather than offering its own retry.
- non-empty -> the real grid, each pack an `.e-pack-card` button showing its `name`; clicking one calls `set("pack", p.id)`, which round-trips through `onChange` into `doc.settings.cursor.pack` and is persisted immediately (same save path as every other field). The selected card gets the `on` class via `settings.pack === p.id`.

**Import.** The "Import pack..." button (`.e-upload-dashed`, `IconFolderPlus`) opens the Tauri dialog plugin's folder picker (`{ directory: true, multiple: false }`). On a chosen folder, calls `importCursorPack(dir)`:
- On success: appends the returned `CursorPackInfo` to local `packs` (so it appears in the grid without a re-fetch; `prev ?? []` covers appending while `packs` was still `null`/loading) and immediately selects it (`set("pack", info.id)`).
- On failure: shows the rejection message (or a generic fallback if it isn't a string) via the shared `.e-errline` class (also used by `AiPanel`'s Engine-picker error line) below the grid, wrapped in `AnimatePresence` (design/premium-pass D6, 0.14s opacity/y-4) so it fades in/out rather than popping.
- While in flight, the button is disabled and its label switches to "Importing...".

**Cursor style.** A 3-way `Picker` (`STYLE_OPTS`) over all three `CursorStyle`s - `system` (keep the recording's baked-in OS cursor), `enhanced` (redraw a synthetic pointer), `hidden` (none) - writing `style` via `set`. Exposing `system` here is what lets a clip recorded in System return to its original cursor in the editor; the old 2-way `Switch` could only reach `enhanced`/`hidden`. Everything below (click-bounce, the pack grid + import, and every slider) is gated behind `settings.style === "enhanced"`, since none of it affects the System or Hidden cursor.

**Reset.** Restores every field to `DEFAULT_CURSOR_SETTINGS` (below), including `pack: "default"` - does *not* touch the locally fetched `packs` list (an imported pack stays visible in the grid after a reset; only the *selection* reverts to Default).

**Sliders.** Cursor Size (`0.4`-`3.0`), Cursor Smoothness (`0.0`-`1.0`), Path Idealization (`0.0`-`1.0`), Motion Trail Blur (`0`-`1`), Click Bounce Intensity (`0.1`-`1`) - each a direct `set(field, v)` on `Slider`'s `onChange`. *Cursor Smoothness* (`settings.smoothness`, default `0.6`) drives the Rust-side follow low-pass alpha (`CursorSettings::follow_alpha`, `0.75 - 0.65 * smoothness`): `0` is snappy/raw cursor tracking, `1` is a glassy, heavily-damped glide. *Path Idealization* (`settings.path_idealize`, default `0.0`) straightens wandering mouse movement into clean strokes between clicks on the export side (`Cursor::set_idealize`); `0` leaves the recorded path untouched.

### Notes

- Selecting or importing a pack takes effect in both the export and the live editor preview: `Editor.tsx` passes `doc.settings.cursor` straight through to `Stage`, and `useEditorData`'s `cursorSprites` fetch is keyed on `doc?.settings.cursor.pack` so it re-decodes sprites exactly when the pack changes (not on unrelated edits).
- No client-side validation of the picked folder happens here - `importCursorPack` (Rust) does all validation (recognized filenames, real PNG bytes, `hotspots.json` shape) and this panel just surfaces whatever error string comes back.

## DEFAULT_CURSOR_SETTINGS

```ts
export const DEFAULT_CURSOR_SETTINGS: CursorSettings
```

Mirrors the Rust `CursorSettings::default()` (`settings/model.rs`) field-for-field, including
`style: "system"`. A Task 26 audit found this panel's Reset button wrote `style: "enhanced"`
here instead, silently diverging from the backend default on every reset; extracted to a named,
independently-testable constant so the two can't drift apart unnoticed again.

### Used by

- `src/editor/Editor.tsx` - rendered for the `"cursor"` rail tab, wired to `doc.settings.cursor` / `saveDocSettings`
