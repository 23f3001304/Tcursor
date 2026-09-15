# src-tauri/src/platform/windows/capture/target.rs

What there is to capture and where it sits: the enumeration of monitors and application windows, and the size and origin of one of them, in the same absolute coordinates the `WH_MOUSE_LL` hook reports.

Batch C1 put the two halves in one file on purpose. `get_target_bounds` and the capture launchers must resolve the SAME target the same way; they once did not, and `display:N` took another monitor's origin and shifted every cursor point. Enumeration is what names the ids those functions are then handed, so it belongs beside them. The whole module is Windows-only (`platform/windows/` is `#[cfg(windows)]`), which is what got the unguarded `Monitor::enumerate` and `Monitor::primary` calls out of `commands.rs` and `session/record/`.

Every id is a `ports::capture::TargetId`, never a hand-parsed string: `TargetId::from_arg` is the one place `window:0x...` and `display:N` are read, and an unparseable id resolves to `Primary` exactly as the four hand-written parsers used to fall through to `Monitor::primary()`.

## get_target_bounds

```rust
pub(super) fn get_target_bounds(target_id: Option<&str>) -> CaptureGeometry
```

The `ports::capture::CaptureGeometry` (`w`, `h`, `origin_x`, `origin_y`) for `target_id`. It is the port type itself, not a private tuple, so `CapturePort::bounds` returns this value unchanged and no call site can re-order the four numbers:

- `TargetId::Window` - `visible_rect` on that HWND (the DWM extended frame bounds, which is the rectangle WGC actually captures; `GetWindowRect` only as a fallback), width and height floored at 100.
- `TargetId::Display(n)` - `display_bounds(n)`: the monitor `windows_capture::monitor::Monitor::from_index(n)` names, correlated to its Win32 rect by GDI device name.
- `TargetId::Primary`, and any branch above that found nothing - `primary_bounds()`.

## primary_bounds

```rust
fn primary_bounds() -> CaptureGeometry
```

The primary display at the origin, and finally a hard `1920x1080` at `(0, 0)` so a caller always gets a usable rectangle. Every other branch of `get_target_bounds` falls through to it rather than failing: a take with a slightly wrong rectangle is recoverable, a take that would not start is not.

## display_bounds

```rust
fn display_bounds(index: usize) -> Option<CaptureGeometry>
```

The Win32 rect of the monitor `Monitor::from_index(index)` names, found by walking `EnumDisplayMonitors` and matching `MONITORINFOEXW::szDevice` against that monitor's device name.

*Why the device name rather than indexing `EnumDisplayMonitors` directly:* the two enumerations are in DIFFERENT orders, so on a multi-monitor desktop `display:N` could take another monitor's origin and shift every cursor point by the delta between them - the "cursor drawn in the wrong place" regression. Matching by device name keeps the video and the origin on the same screen, and keeps this function agreeing with `list_targets` and `gpu::record::start_capture`, which both index monitors the same way.

## MonCtx

```rust
struct MonCtx { want: Vec<u16>, bounds: Option<CaptureGeometry> }
```

The `LPARAM` context `enum_mon_cb` is handed: the UTF-16 device name to match and the slot the match is written into. `EnumDisplayMonitors` takes a bare `extern "system"` callback with one `isize` of user data, so the search state has to be a struct behind a raw pointer.

## enum_mon_cb

```rust
unsafe extern "system" fn enum_mon_cb(hmon: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL
```

The `EnumDisplayMonitors` callback. Reads each monitor's `MONITORINFOEXW`, compares `szDevice` up to its NUL with `MonCtx::want`, and on a match stores `rcMonitor` as a `CaptureGeometry` (`w`, `h`, `origin_x: left`, `origin_y: top`). Always returns `BOOL(1)`: the walk runs to completion rather than stopping at the match, which costs one pass over at most a handful of monitors and keeps the callback free of early-exit state.

## capture_window_size

```rust
pub(super) fn capture_window_size(hwnd: HWND) -> (u32, u32)
```

The size both capture launchers (`gpu::record::start_capture` and `legacy::wgc_source`) give WGC for a window target: `GetWindowRect`, floored at 100 x 100, falling back to 1920x1080 when the call fails.

*Why this is NOT `visible_rect`.* The two are deliberately different rectangles. The launchers want the OUTER rect because that is roughly what WGC frames a window by, and the value is only an estimate anyway - the encoder is sized from the first real frame. `get_target_bounds` wants the INNER rect, because that is the coordinate space mouse samples are measured against. `pub(super)` so both the `gpu` and `legacy` submodules share one copy and the two launchers cannot drift apart.

## visible_rect

```rust
fn visible_rect(hwnd: HWND) -> Option<RECT>
```

The rectangle of a window's PIXELS: `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)`, the visible frame without the invisible resize borders, falling back to `GetWindowRect` when DWM declines or returns an empty rectangle. *Why (2026-09-14):* `GetWindowRect` includes the invisible borders, so a maximized window read 1944x1104 at -12,-12 while WGC delivered 1920x1080 at 0,0; a take that stored the outer rectangle as its `ScreenInfo` drew every cursor and click 12 px down and to the right of where it happened. The extended frame bounds are what `windows_capture` sizes its frames from, so the origin the mouse samples are measured against is the origin of the pixels again.

## list_targets

```rust
pub(super) fn list_targets() -> Vec<CaptureTarget>
```

Everything the user can record, monitors first and then application windows, in the order the HUD's target sheet shows them. When no monitor enumerates at all, one synthetic `Display(0)` "Primary Display" entry stands in so the picker is never empty - the same fallback the old `commands::list_displays` had, in the same position (before the windows are appended).

### Used by

- `platform/windows/capture/mod.rs` - `Win32Capture::list_targets`, verbatim.
- `commands.rs` - `list_displays` converts the result down to the frontend's `DisplayInfo`.

## displays

```rust
fn displays() -> Vec<CaptureTarget>
```

One `CaptureTarget` per `Monitor::enumerate()` entry, `TargetId::Display(i)` by enumeration index. The label carries the size and the primary flag on the end as `(WxH, Primary)`, `(Primary)` or `(WxH)`, which is what the HUD's `parseTarget` splits back out to draw each monitor to scale. An enumeration failure is an empty list, not an error: `list_targets` substitutes the fallback entry.

## enum_windows_cb

```rust
unsafe extern "system" fn enum_windows_cb(hwnd: HWND, lparam: LPARAM) -> BOOL
```

The `EnumWindows` callback, appending to the `Vec<CaptureTarget>` behind `lparam`. Keeps a window only if it is visible, has a non-empty title, and is not a `WS_EX_TOOLWINDOW`; then drops the four titles that are always noise (`Program Manager`, `Settings`, `TCursor` itself, and the `MSCTFIME` IME host). The id is `TargetId::Window(hwnd)`, which renders back as `window:0x<hex>` - the exact string the take stores and `TargetId::from_arg` reads.

## app_windows

```rust
fn app_windows() -> Vec<CaptureTarget>
```

Runs `EnumWindows` with `enum_windows_cb` over a local `Vec`, handed in as the callback's `LPARAM`.
