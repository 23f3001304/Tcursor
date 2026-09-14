# src-tauri/src/session/record/target_bounds.rs

Where a capture target actually sits on the desktop: the size and the origin of a monitor or an application window, in the same absolute coordinates the `WH_MOUSE_LL` hook reports.

Split out of `video_sink.rs` (which picks and starts the pipeline, and is at the 200-line cap) when mid-take display switching landed, because the origin is now asked for twice: once when a take starts, to fill the take's one `ScreenInfo`, and again on every `switch_display`, to build the `events::remap::Remap` that puts later mouse samples where the pixels are.

## get_target_bounds

```rust
pub fn get_target_bounds(target_id: Option<&str>) -> (u32, u32, i32, i32)
```

`(w, h, origin_x, origin_y)` for `target_id`:

- `window:0x<hex hwnd>` - `visible_rect` on that HWND (the DWM extended frame bounds, which is the rectangle WGC actually captures; `GetWindowRect` only as a fallback), width and height floored at 100.
- `display:N` - the monitor `windows_capture::monitor::Monitor::from_index(N)` names, correlated to its Win32 rect by GDI device name.
- anything else, including `None` - the primary display at the origin, and finally a hard `(1920, 1080, 0, 0)` so a caller always gets a usable rectangle.

*Why `display:N` goes through the device name rather than indexing `EnumDisplayMonitors` directly:* the two enumerations are in DIFFERENT orders, so on a multi-monitor desktop `display:N` could take another monitor's origin and shift every cursor point by the delta between them - the "cursor drawn in the wrong place" regression. Matching by `MONITORINFOEXW::szDevice` keeps the video and the origin on the same screen, and keeps this function agreeing with `list_displays` and `gpu_record::start_capture`, which both index monitors the same way.

Non-Windows builds fall straight through to the primary-monitor branch.

## visible_rect

```rust
fn visible_rect(hwnd: HWND) -> Option<RECT>
```

Windows only. The rectangle of a window's PIXELS: `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)`, the visible frame without the invisible resize borders, falling back to `GetWindowRect` when DWM declines or returns an empty rectangle. *Why (2026-09-14):* `GetWindowRect` includes the invisible borders, so a maximized window read 1944x1104 at -12,-12 while WGC delivered 1920x1080 at 0,0; a take that stored the outer rectangle as its `ScreenInfo` drew every cursor and click 12 px down and to the right of where it happened. The extended frame bounds are what `windows_capture` sizes its frames from, so the origin the mouse samples are measured against is the origin of the pixels again.
