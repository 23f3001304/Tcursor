# src/editor/shell/TopBar.tsx

Editor top bar rendered at the top of the editor view. The bar is a window drag region; it shows a back-to-recorder button, the TCursor brand mark and project name, real Undo/Redo buttons, an Export button (with a `Spin` + progress while exporting), and window minimize/maximize/close controls. Holds one piece of local state: whether the window is maximized.

## TopBar

```tsx
export function TopBar({ proj, exporting, pct, onOpenExport, onClose, onUndo, onRedo, canUndo, canRedo }: {
  proj: string; exporting: boolean; pct: number; onOpenExport: () => void; onClose: () => void;
  onUndo: () => void; onRedo: () => void; canUndo: boolean; canRedo: boolean;
}): JSX.Element
```

Renders the editor top bar with navigation, branding, and export controls.

### Props

- `proj: string` - the project folder's basename (last path segment). *Why:* gives the user context about which recording is open without showing the full path.
- `exporting: boolean` - whether an export pipeline is running. *Why:* disables the Export button and switches it to a spinner + percentage display to prevent double-submission. Reflects an export started from `ExportDialog`, so this mini bar still tracks progress even after the dialog itself has been closed.
- `pct: number` - export progress percentage (0-100). *Why:* shown inline in the Export button while `exporting` is true.
- `onOpenExport: () => void` - called when the Export button is clicked. *Why:* opens `ExportDialog` (owned by `Editor`) rather than exporting immediately - the dialog collects `ExportSettings` and calls `exportProject` itself once the user confirms.
- `onClose: () => void` - called when the **back-arrow** button is clicked. *Why:* "back to recorder" is owned by `App` (via `Editor`'s `onClose`), which switches the view back to the HUD. The close (X) button does not use this - it quits the app.
- `onUndo: () => void` / `onRedo: () => void` - run one undo/redo step. Wired in `Editor.tsx` to `useEditHistory`'s `undo()`/`redo()`.
- `canUndo: boolean` / `canRedo: boolean` - whether a step is actually available in each direction; disables the corresponding button rather than hiding it, so the bar's width never shifts as history fills or empties.

### Behavior

**Back button.**
Calls `onClose`. No confirmation -- unsaved edits and ongoing exports are the caller's responsibility to handle.

**Export button.**
When `exporting` is false: renders `IconDownload` + "Export" text and calls `onOpenExport` on click.
When `exporting` is true: renders `<Spin size={15}>` + `{pct}%` and is `disabled`. *Why disabled during export:* `exportProject` is a one-at-a-time pipeline; a second concurrent call is not supported.

**Undo / Redo.**
Undo (`IconArrowBackUp`, title "Undo (Ctrl+Z)") calls `onUndo` and is `disabled={!canUndo}`; Redo (`IconArrowForwardUp`, title "Redo (Ctrl+Shift+Z)") calls `onRedo` and is `disabled={!canRedo}`. Both are icon-only (no text label) - the tooltip carries the keyboard shortcut. No GitHub button exists in this bar (an earlier build had a decorative, handlerless GitHub stub here; it was dropped rather than kept as dead weight).

**Branding.**
`.e-brand` renders the cursor-icon mark (`.e-mark`), the "TCursor" wordmark, and the `proj` name in a lighter `.e-proj` span.

**Window controls + drag.**
The bar has `data-tauri-drag-region`, and CSS sets `pointer-events: none` on its non-button children, so dragging the empty bar / brand area moves the window while the buttons stay clickable.
- **Minimize** (`IconMinus`) calls `getCurrentWindow().minimize()`.
- **Maximize** (`IconMaximize`) toggles a local `maxed` flag: on it `setSize`s to the full screen and `setPosition`s to (0,0); off it `setSize`s back to the comfortable windowed size and `center()`s. *Why setSize rather than `maximize()`:* native maximize no-ops on this transparent window, but `setSize` works.
- **Close** (`IconX`, red hover) calls `getCurrentWindow().close()`, which closes the sole window and so quits the app.

*Why both a back arrow and a close:* the back arrow returns to the recorder HUD (`onClose`); the X quits, matching where users expect a window-close.

### Notes

- TopBar holds one `useState` (`maxed`) and no effects; it calls `getCurrentWindow()` for the minimize/maximize/close handlers.
- The `pct` prop is a raw number; the `%` character is appended in the JSX, not in the prop contract.
