# src-tauri/src/ports/system.rs

The small OS facts the app asks for that are not capture, input or audio: the display's refresh rate, the desktop's dark-mode preference, and whether a window of ours is hidden from screen capture.

Three methods, not six. Target enumeration lives on `CapturePort` instead, so starting a capture and asking for a target's rectangle resolve it the same way. The app icon and the taskbar progress bar are not here at all: they are pure Tauri calls (`set_icon`, `set_progress_bar`) that Tauri implements on all three platforms, so they are shell code with no port.

## WindowHandle

```rust
pub struct WindowHandle(pub isize);
```

An opaque OS window handle, resolved from a `tauri::WebviewWindow` at the composition root.

*Why not take the `WebviewWindow` itself.* The recording core keeps Tauri types out on purpose - `Notify` is an `Arc<dyn Fn(&str)>` for the same reason - so a port that took a Tauri window would be the one place the boundary leaked.

*Why not resolve the window inside the port.* Both of today's callers reach for `get_webview_window("main")`. Studio is a mode inside this same binary with its own launcher window, and that second window needs the same capture-exclusion treatment, so the window has to be a parameter. On Windows the `isize` is the raw HWND the exclusion shim already takes.

## SystemPort

```rust
pub trait SystemPort: Send + Sync
```

The desktop facts.

## SystemPort::primary_refresh_hz

```rust
fn primary_refresh_hz(&self) -> u32
```

The refresh rate a take captures and encodes at, clamped by the caller (to 60 today). Infallible: the Windows shim answers 60 when it cannot read the display, and so should any other.

## SystemPort::os_prefers_dark

```rust
fn os_prefers_dark(&self) -> bool
```

The desktop's own light/dark preference, which is what `ThemeMode::System` resolves through. Worth a spike before a second adapter is written: Tauri 2's `Window::theme()` may be able to replace the platform read on all three platforms, in which case this method stops being a port at all.

## SystemPort::exclude_from_capture

```rust
fn exclude_from_capture(&self, window: WindowHandle, exclude: bool) -> bool
```

Hides a window from screen capture and screenshots, or restores it.

Returns whether the window NOW HAS the requested affinity, read back rather than assumed. *Why that and not "did the call succeed":* the owner saw the take pill in a display capture with the flag already set, so callers re-apply it after every HUD window morph and need the truth each time. A `bool` that only reported the setter's return value would have said everything was fine throughout that bug.
