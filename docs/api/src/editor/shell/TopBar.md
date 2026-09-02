# src/editor/shell/TopBar.tsx

Editor top bar rendered at the top of the editor view. The bar is a window drag region; it shows a back-to-recorder button, the TCursor brand mark and project name, real Undo/Redo buttons, an Export button (with a `Spin` + progress while exporting), a Project settings gear button, and window minimize/maximize/close controls. Holds one piece of local state: whether the window is maximized.

## TopBar

```tsx
export function TopBar({ proj, exporting, pct, onOpenExport, onOpenSettings, onClose, onUndo, onRedo, canUndo, canRedo, brandState }: {
  proj: string; exporting: boolean; pct: number; onOpenExport: () => void; onOpenSettings: () => void; onClose: () => void;
  onUndo: () => void; onRedo: () => void; canUndo: boolean; canRedo: boolean; brandState: MarkState;
}): JSX.Element
```

Renders the editor top bar with navigation, branding, and export controls.

### Props

- `proj: string` - the project folder's basename (last path segment). *Why:* gives the user context about which recording is open without showing the full path.
- `exporting: boolean` - whether an export pipeline is running. *Why:* disables the Export button and switches it to a spinner + percentage display to prevent double-submission. Reflects an export started from `ExportDialog`, so this mini bar still tracks progress even after the dialog itself has been closed.
- `pct: number` - export progress percentage (0-100). *Why:* shown inline in the Export button while `exporting` is true, and passed straight through to the brand mark (`TcursorMark`'s `pct` prop) for its exporting flow-speed mapping.
- `onOpenExport: () => void` - called when the Export button is clicked. *Why:* opens `ExportDialog` (owned by `Editor`) rather than exporting immediately - the dialog collects `ExportSettings` and calls `exportProject` itself once the user confirms.
- `onOpenSettings: () => void` (Task 35) - called when the gear button (next to Export) is clicked. *Why:* opens `EditorSettingsDialog` (owned by `Editor`, same pattern as `onOpenExport`) - project-scoped zoom-defaults/screen/interface settings that had no editor surface before.
- `onClose: () => void` - called when the **back-arrow** button is clicked. *Why:* "back to recorder" is owned by `App` (via `Editor`'s `onClose`), which switches the view back to the HUD. The close (X) button does not use this - it quits the app.
- `onUndo: () => void` / `onRedo: () => void` - run one undo/redo step. Wired in `Editor.tsx` to `useEditHistory`'s `undo()`/`redo()`.
- `canUndo: boolean` / `canRedo: boolean` - whether a step is actually available in each direction; disables the corresponding button rather than hiding it, so the bar's width never shifts as history fills or empties.
- `brandState: MarkState` (Task 39) - the living brand mark's state, fully pre-computed by `Editor.tsx` (see `Editor.md`'s `brandState` note) since it's the component with `exporting`/`running`/`doc.settings.ui.animated_brand` all already in scope. `TopBar` just threads it (plus `pct`) into `TcursorMark` - it derives nothing itself.

### Behavior

**Back button.**
Title "Return to the recorder" (Task 36 copy pass - reads as a sentence rather than a label). Calls `onClose`. No confirmation -- unsaved edits and ongoing exports are the caller's responsibility to handle.

**Export button.**
`motion.button.e-export` (`whileHover: scale 1.03`, `whileTap: scale 0.97`, both omitted while `exporting` since a disabled button shouldn't invite a press; the CSS `:hover`/`:active` `transform` this used to carry was removed so Motion's inline transform is the only one setting it). design/premium-pass D3 switched the transition from a `0.12s` tween to a spring (`type: "spring", stiffness: 500, damping: 30` - the same spring `Transport`'s Play button uses, `PLAY_SPRING`) for a consistent press feel across the app's two hero buttons, and added `--e-inset-hi` to `.e-export`'s box-shadow (both at rest and on hover) in place of a bespoke inset value.
When `exporting` is false: renders `IconDownload` + "Export" text and calls `onOpenExport` on click.
When `exporting` is true: renders `<Spin size={15}>` + `{pct}%` and is `disabled`. *Why disabled during export:* `exportProject` is a one-at-a-time pipeline; a second concurrent call is not supported.

**Project settings.**
`.e-gst` gear button (`IconSettings`, title "Project settings"), sitting right after the Export button and before the window-controls divider. Calls `onOpenSettings` on click - no local state, no disabled condition (unlike Export, opening it mid-export is fine; the dialog's own writes go through `saveDocSettings` regardless of `exporting`).

**Undo / Redo.**
Undo (`IconArrowBackUp`, title "Undo (Ctrl+Z)") calls `onUndo` and is `disabled={!canUndo}`; Redo (`IconArrowForwardUp`, title "Redo (Ctrl+Shift+Z)") calls `onRedo` and is `disabled={!canRedo}`. Both are icon-only (no text label) - the tooltip carries the keyboard shortcut. No GitHub button exists in this bar (an earlier build had a decorative, handlerless GitHub stub here; it was dropped rather than kept as dead weight).

**Branding.**
`.e-brand` renders the TCursor brand mark (`.e-mark`, via `TcursorMark state={brandState} pct={pct}` - see `docs/api/src/lib/TcursorMark.md`), the "TCursor" wordmark, and the `proj` name in a lighter `.e-proj` span. `.e-mark`'s own square-badge styling (size, background, radius) is unchanged; only its glyph swapped from a generic cursor icon to the brand mark. `brandState` (Task 39) makes the mark flow while exporting and tint+flow while the AI director is directing, gated by `doc.settings.ui.animated_brand` - see `Editor.md`.

**Window controls + drag.**
The bar has `data-tauri-drag-region`, and CSS sets `pointer-events: none` on its non-button children, so dragging the empty bar / brand area moves the window while the buttons stay clickable.
- **Minimize** (`IconMinus`) calls `getCurrentWindow().minimize()`.
- **Maximize** (`IconMaximize`) toggles a local `maxed` flag: on it `setSize`s to the full screen and `setPosition`s to (0,0); off it `setSize`s back to the comfortable windowed size and `center()`s. *Why setSize rather than `maximize()`:* native maximize no-ops on this transparent window, but `setSize` works.
- **Close** (`IconX`, red hover) calls `getCurrentWindow().close()`, which closes the sole window and so quits the app.

*Why both a back arrow and a close:* the back arrow returns to the recorder HUD (`onClose`); the X quits, matching where users expect a window-close.

### Notes

- TopBar holds one `useState` (`maxed`) and no effects; it calls `getCurrentWindow()` for the minimize/maximize/close handlers.
- The `pct` prop is a raw number; the `%` character is appended in the JSX, not in the prop contract.
