# src-tauri/src/win/mod.rs

MODULE OVERVIEW: The `win` module wraps Windows-specific platform APIs that the rest of the backend needs but should not call directly. Each submodule is a thin, focused shim: it performs one OS query or side effect, provides a non-Windows stub so the crate compiles on all targets, and returns a plain Rust value. Together the four submodules handle capture-exclusion of the HUD window, primary display refresh-rate detection, bundled-binary discovery and `Command` construction for ffmpeg/ffprobe, and OS dark-mode resolution. All four are called at app startup or at the start of a recording; none runs on a background thread of its own.

## theme

Reads the Windows registry dark-mode preference and resolves a `ThemeMode` setting to a concrete boolean. The result drives cursor sprite inversion and HUD coloring. Key items: `os_prefers_dark` (queries `HKCU\...\Themes\Personalize\AppsUseLightTheme`, returns `false` on any failure), `resolve_dark` (maps `ThemeMode::Light/Dark/System` to a `bool`, calling `os_prefers_dark` only for `System`).
