# src/editor/TopBar.tsx

Editor top bar rendered at the top of the editor view. The bar is a window drag region; it shows a back-to-recorder button, the TCursor brand mark and project name, undo/redo stubs, a GitHub stub, an Export button (with a `Spin` + progress while exporting), and window minimize/maximize/close controls. Holds one piece of local state: whether the window is maximized.

## TopBar

```tsx
export function TopBar({ proj, exporting, pct, onExport, onClose }: {
  proj: string; exporting: boolean; pct: number; onExport: () => void; onClose: () => void;
}): JSX.Element
```

Renders the editor top bar with navigation, branding, and export controls.

### Props

- `proj: string` - the project folder's basename (last path segment). *Why:* gives the user context about which recording is open without showing the full path.
- `exporting: boolean` - whether an export pipeline is running. *Why:* disables the Export button and switches it to a spinner + percentage display to prevent double-submission.
- `pct: number` - export progress percentage (0-100). *Why:* shown inline in the Export button while `exporting` is true.
- `onExport: () => void` - called when the Export button is clicked. *Why:* export state is owned by `Editor`.
- `onClose: () => void` - called when the **back-arrow** button is clicked. *Why:* "back to recorder" is owned by `App` (via `Editor`'s `onClose`), which switches the view back to the HUD. The close (X) button does not use this - it quits the app.

### Behavior

**Back button.**
Calls `onClose`. No confirmation -- unsaved edits and ongoing exports are the caller's responsibility to handle.

**Export button.**
When `exporting` is false: renders `IconDownload` + "Export" text and calls `onExport` on click.
When `exporting` is true: renders `<Spin size={15}>` + `{pct}%` and is `disabled`. *Why disabled during export:* `exportProject` is a one-at-a-time pipeline; a second concurrent call is not supported.

**Stub buttons.**
Undo (`IconArrowBackUp`) and Redo (`IconArrowForwardUp`) are rendered with `disabled` and no handler. GitHub icon (`IconBrandGithub`) has no handler. These occupy their positions now for layout stability.

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
