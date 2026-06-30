# src-tauri/src/win/display.rs

Single-function module that queries the primary display's refresh rate. The result is used as the target capture and encode frame rate so TCursor matches the display's native cadence without hard-coding 60 fps.

## primary_refresh_hz

```rust
pub fn primary_refresh_hz() -> u32
```

Returns the refresh rate of the primary display in Hz. On non-Windows targets always returns `60`.

### Inputs

None. The function reads the primary display settings from the OS.

### Returns

`u32` - the display's reported `dmDisplayFrequency` when the Win32 call succeeds and the value is at least 24. Falls back to `60` if the call fails or returns a value below 24. *Why 60 as fallback:* a widely supported safe default that matches the majority of displays; values below 24 indicate a partial or zeroed `DEVMODEW`, not a real 1 or 8 Hz display.

### Implementation

1. (Windows only) Allocate a zero-initialised `DEVMODEW` with `dmSize` set to `sizeof(DEVMODEW)`. *Why `dmSize` must be set:* `EnumDisplaySettingsW` checks this field to determine the caller's struct version; leaving it zero causes the call to fail.
2. Call `EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm)` inside an `unsafe` block. `PCWSTR::null()` selects the primary display.
3. If the call returns true and `dm.dmDisplayFrequency >= 24`, return `dm.dmDisplayFrequency`.
4. Otherwise return `60`.
5. (non-Windows) Return `60` unconditionally via the `#[cfg(not(windows))]` stub.

### Used by

- `src-tauri/src/session/recorder.rs` - calls `.min(60)` on the result to cap WGC capture at 60 fps (the maximum TCursor encodes at), preventing unnecessary CPU load on 120/144 Hz displays
- `src-tauri/src/export/run.rs` - reads the primary refresh rate (capped at 60) to set the encode frame rate for exported video
