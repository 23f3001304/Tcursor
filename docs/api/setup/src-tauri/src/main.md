# setup/src-tauri/src/main.rs

The Setup app's entry point. Registers the six commands `setup/ui/app.js` calls and installs the
window-close guard; everything else lives in `install`, `progress` and `log`.

The window itself is configured in `setup/src-tauri/tauri.conf.json`: 480x440, not resizable, no
decorations, centred, transparent (the UI draws its own 14px rounded frame). Nothing is persisted,
so the window never "remembers" a position or size.

Windows conventions in a frameless shell, and where each is handled:

- **Alt-Tab and Task Manager name.** `productName` and the window `title` are both `TCursor Setup`.
  Task Manager shows the exe's `FileDescription`, which `tauri_build` takes from the crate's
  Cargo `description` - which is why that field is the product name and not a sentence about it.
- **Drag.** The header carries `data-tauri-drag-region`; the capability grants
  `core:window:allow-start-dragging`.
- **Esc and Alt+F4.** Esc is a `keydown` handler in `app.js` that calls `window.close()`; Alt+F4
  reaches the same place through the OS. Both therefore arrive as `CloseRequested`, and the guard
  below is the only place that decides.

## main

```rust
fn main()
```

Logs the start (which also fixes the log's time zero), then builds the Tauri app with the command
handlers and one window-event hook.

The hook is the close guard. A `CloseRequested` while an install is running is refused with
`api.prevent_close()` and answered by emitting `setup://close-request`, which the UI turns into a
one-line "Installing. Close anyway?" strip with Yes and No. Once the user picks Yes,
`install::allow_close` sets the flag this guard reads and closes for real.

*Why the question is asked in the window rather than by `MessageDialog`:* an OS dialog in front of
a frameless, branded, undecorated window is the exact seam that makes an installer look assembled
from parts. It is also an extra Tauri plugin for a single yes/no.

*Why the guard reads two flags.* `busy()` alone would re-intercept the close that the user just
approved, so `close_allowed()` is the latch that lets it through. Neither is ever reset: there is
no path back from "the user chose to close".

When no install is running, `CloseRequested` is not touched at all, which is what makes the Done
screen's auto-close and the plain X button work with no special case.
