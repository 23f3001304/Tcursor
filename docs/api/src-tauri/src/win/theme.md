# src-tauri/src/win/theme.rs

Reads the OS dark-mode preference and resolves a `ThemeMode` setting to a concrete dark/light boolean. Used to decide whether to invert cursor sprites and apply dark-theme coloring to the HUD and exports.

## os_prefers_dark

```rust
pub fn os_prefers_dark() -> bool
```

Queries the Windows registry to determine whether the user has enabled dark mode for applications. On non-Windows targets always returns `false`.

### Inputs

None. The function reads from the current user's registry hive directly.

### Returns

`bool` - `true` when the Win32 registry read succeeds AND `AppsUseLightTheme == 0` (meaning dark mode is active). Returns `false` if the registry call fails for any reason (key absent, permission denied, unexpected data type). *Why `false` as the failure value:* light mode is the TCursor default, so failing to determine OS preference degrades gracefully to the light theme.

### Implementation

1. (Windows only) Define the registry key path `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` and value name `AppsUseLightTheme` as wide-string constants via `windows::core::w!`.
2. Declare a `u32` buffer and a 4-byte size for the DWORD read.
3. Call `RegGetValueW(HKEY_CURRENT_USER, subkey, value, RRF_RT_REG_DWORD, None, &mut data, &mut size)` inside an `unsafe` block.
4. Return `result.is_ok() && data == 0`. *Why `== 0`:* the registry value name is `AppsUseLightTheme`; a value of 0 means light mode is OFF, i.e. dark mode is active.
5. (non-Windows) Return `false` via the `#[cfg(not(windows))]` stub.

### Behaviors

- `system_does_not_panic` - calls `resolve_dark(ThemeMode::System)` (which calls `os_prefers_dark`) and asserts no panic; the result is not checked since it depends on the test machine's OS setting.

### Used by

- `src-tauri/src/win/theme.rs` (`resolve_dark`) - called only when `ThemeMode::System` is active

## resolve_dark

```rust
pub fn resolve_dark(theme: ThemeMode) -> bool
```

Maps a `ThemeMode` variant to a concrete dark/light boolean, delegating to `os_prefers_dark` for `System`.

### Inputs

- `theme: ThemeMode` - the user's configured theme preference. *Why `ThemeMode` by value rather than reference:* it is `Copy`; passing by value avoids a reference without cost.

### Returns

`bool` - `false` for `Light`; `true` for `Dark`; `os_prefers_dark()` for `System`.

### Implementation

1. `match theme`: three exhaustive arms with no default. *Why no default arm:* forces a compile error if a new `ThemeMode` variant is added without updating this function.

### Behaviors

- `light_is_not_dark` - asserts `resolve_dark(ThemeMode::Light) == false`.
- `dark_is_dark` - asserts `resolve_dark(ThemeMode::Dark) == true`.
- `system_does_not_panic` - asserts the `System` branch completes without panicking.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - called once per export with `settings.ui.theme`; the boolean drives cursor sprite inversion (dark theme = invert light sprites) and any dark-mode color adjustments in the export pipeline
