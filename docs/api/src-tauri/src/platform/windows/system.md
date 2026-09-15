# src-tauri/src/platform/windows/system.rs

`SystemPort` on Windows: the three desktop facts that are neither capture, input nor audio - the primary display's refresh rate, the desktop dark-mode preference, and whether one of our windows is hidden from screen capture.

The bodies live here, not behind a forward. Until cross-platform Phase 1, Batch C3 (2026-09-15) they were three shims under `win/` (`win/sys/display.rs`, `win/theme.rs`, `win/sys/capture_exclusion.rs`), each with a `#[cfg(not(windows))]` stub of its own. `platform/windows/` only compiles on Windows, so the stubs are gone and the portable fallbacks (60 Hz, light, `false`) live once in `platform/mod.rs`. `win/` was deleted in the same change. The `ThemeMode` mapping that used to sit beside the registry read (`resolve_dark`) is NOT here: only the registry read is platform code.

## Win32System

```rust
pub struct Win32System;
```

The desktop facts. A unit struct; all three calls are stateless queries into the OS, so there is nothing to hold.

## Win32System::primary_refresh_hz

```rust
fn primary_refresh_hz(&self) -> u32
```

The primary display's refresh rate in Hz, used as the target capture and encode frame rate so a take matches the display's native cadence instead of hard-coding 60.

### Returns

`u32` - `dmDisplayFrequency` when the Win32 call succeeds and the value is at least 24; otherwise `60`. *Why 60 as the fallback:* a widely supported safe default that matches most displays. *Why the 24 floor:* a value below 24 means a partial or zeroed `DEVMODEW`, not a real 1 Hz or 8 Hz display.

### Implementation

1. Allocate a zero-initialised `DEVMODEW` with `dmSize` set to `sizeof(DEVMODEW)`. *Why `dmSize` must be set:* `EnumDisplaySettingsW` reads it to decide the caller's struct version, and a zero there makes the call fail.
2. Call `EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm)` inside an `unsafe` block. `PCWSTR::null()` selects the primary display.
3. Return `dm.dmDisplayFrequency` if the call returned true and the value is at least 24, else `60`.

The caller still clamps the result to 60 (`crate::platform::primary_refresh_hz().min(60)` at every call site). That clamp is a recording policy, not a platform fact, so it stays where it is.

### Used by

- `src-tauri/src/platform/mod.rs` (`primary_refresh_hz`) - the portable free function every call site goes through. Its callers are `session/record/recorder.rs`, `session/record/switch_display.rs`, `export/pipeline/exporter.rs` and `export/preview/mod.rs`.

## Win32System::os_prefers_dark

```rust
fn os_prefers_dark(&self) -> bool
```

Whether the user has enabled dark mode for applications, read from the registry.

### Returns

`bool` - `true` when the read succeeds AND `AppsUseLightTheme == 0` (light mode off, so dark mode is on). `false` on any failure: key absent, permission denied, unexpected data type. *Why `false` on failure:* light is the TCursor default, so an unreadable preference degrades to the light theme.

### Implementation

1. Name the key `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` and the value `AppsUseLightTheme` as wide strings via `windows::core::w!`.
2. Declare a `u32` buffer and a 4-byte size for the DWORD read.
3. Call `RegGetValueW(HKEY_CURRENT_USER, subkey, value, RRF_RT_REG_DWORD, None, &mut data, &mut size)` inside an `unsafe` block.
4. Return `result.is_ok() && data == 0`.

Worth a spike before a second adapter is written: Tauri 2's `Window::theme()` may replace the platform read on all three platforms, in which case this stops being a port at all.

### Used by

- `src-tauri/src/platform/mod.rs` (`os_prefers_dark`, and `resolve_dark` through it) - reached only when the user's `ThemeMode` is `System`.

## Win32System::exclude_from_capture

```rust
fn exclude_from_capture(&self, window: WindowHandle, exclude: bool) -> bool
```

Hides a window of ours from every screen-capture path (WGC, BitBlt, DXGI) so the TCursor HUD never appears in the user's own recordings, or opts it back in so the editor can be screenshotted.

### Inputs

- `window: WindowHandle` - the raw HWND as an `isize`, resolved from a `tauri::WebviewWindow` by `crate::shell::window::handle`. *Why a parameter and not a lookup here:* both of today's callers reach for `get_webview_window("main")`, and Studio is a mode inside this same binary with its own launcher window that needs the same treatment, so the port must never pick a window for the caller.
- `exclude: bool` - true to hide the window (the HUD), false to opt it back into capture (the editor).

### Returns

`bool` - whether the window NOW HAS the requested affinity, READ BACK rather than assumed: `SetWindowDisplayAffinity` succeeded AND `GetWindowDisplayAffinity` then reports the value that was asked for. *Why that and not "did the call succeed":* the owner saw the take pill in a display capture although the flag had been set at startup, so the HUD re-applies it after every window morph (`morph.ts` `keepHidden`) and needs the truth each time. A `bool` that only reported the setter's return would have said everything was fine throughout that bug.

### Implementation

1. Choose `WDA_EXCLUDEFROMCAPTURE` if `exclude` else `WDA_NONE`.
2. Rebuild the `HWND` by casting `window.0 as *mut _` and call `SetWindowDisplayAffinity`. *Why unsafe:* a raw Win32 API; the handle came from Tauri in this same process, so the cast is sound.
3. Read the affinity back with `GetWindowDisplayAffinity`; the answer is `set_ok && read_ok && have == want`. A mismatch is logged to stderr as `capture exclusion: wanted ..., window has ...`, so a dev console shows exactly when Windows dropped it.

### Used by

- `src-tauri/src/platform/mod.rs` (`exclude_from_capture`) - the portable free function, called from `lib.rs`'s `setup` at startup and from `commands.rs` (`set_capturable`) at runtime.
