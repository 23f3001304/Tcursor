# src-tauri/src/win/mod.rs

MODULE OVERVIEW: The `win` module wraps Windows-specific platform APIs that the rest of the backend needs but should not call directly. Each submodule is a thin, focused shim: it performs one OS query or side effect, provides a non-Windows stub so the crate compiles on all targets, and returns a plain Rust value. Together the four submodules handle capture-exclusion of the HUD window, primary display refresh-rate detection, bundled-binary discovery and `Command` construction for ffmpeg/ffprobe, and OS dark-mode resolution. All four are called at app startup or at the start of a recording; none runs on a background thread of its own.

## capture_exclusion

Hides the TCursor HUD window from Windows Graphics Capture and other screen-capture APIs so the overlay never appears in the user's own recordings. Key items: `exclude_from_capture` (calls `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on an `isize` HWND, returns a success bool).

## display

Queries the primary display refresh rate to use as the target capture and encode fps, matching the native cadence without hard-coding 60 Hz. Key items: `primary_refresh_hz` (calls `EnumDisplaySettingsW`, falls back to 60 on failure or sub-24 Hz results).

## proc

Discovers the bundled ffmpeg/ffprobe binaries at startup and exposes a `Command` builder that suppresses Windows console-window flicker on every invocation. Key items: `set_ffmpeg_dir` (stores the resolved binary directory into a `OnceLock`), `ffmpeg_candidates` (builds the priority-ordered candidate directory list from exe path and Tauri resource dir), `choose_dir` (returns the first candidate directory containing a given binary name), `init_ffmpeg` (orchestrates discovery and returns a diagnostic string for the log), `ffcmd` (constructs a `Command` for `ffmpeg` or `ffprobe` with `CREATE_NO_WINDOW` on Windows).

## theme

Reads the Windows registry dark-mode preference and resolves a `ThemeMode` setting to a concrete boolean. The result drives cursor sprite inversion and HUD coloring. Key items: `os_prefers_dark` (queries `HKCU\...\Themes\Personalize\AppsUseLightTheme`, returns `false` on any failure), `resolve_dark` (maps `ThemeMode::Light/Dark/System` to a `bool`, calling `os_prefers_dark` only for `System`).
