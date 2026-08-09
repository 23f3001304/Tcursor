# src-tauri/src/win/sys/mod.rs

Submodule overviews for the `sys` group.

## brand_icon

Dynamic app icon + Windows taskbar progress (Task 39). Key items: `set_recording` (swaps the main window's icon between the normal brand mark and a REC-lit variant), `set_export_progress` (drives the taskbar progress bar from an export's percent-complete). Both are graceful no-ops off-Windows or on any failure - see `brand_icon.md`.

## capture_exclusion

Hides the TCursor HUD window from Windows Graphics Capture and other screen-capture APIs so the overlay never appears in the user's own recordings. Key items: `exclude_from_capture` (calls `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` on an `isize` HWND, returns a success bool).

## display

Queries the primary display refresh rate to use as the target capture and encode fps, matching the native cadence without hard-coding 60 Hz. Key items: `primary_refresh_hz` (calls `EnumDisplaySettingsW`, falls back to 60 on failure or sub-24 Hz results).

## proc

Discovers the bundled ffmpeg/ffprobe binaries at startup and exposes a `Command` builder that suppresses Windows console-window flicker on every invocation. Key items: `set_ffmpeg_dir` (stores the resolved binary directory into a `OnceLock`), `ffmpeg_candidates` (builds the priority-ordered candidate directory list from exe path and Tauri resource dir), `choose_dir` (returns the first candidate directory containing a given binary name), `init_ffmpeg` (orchestrates discovery and returns a diagnostic string for the log), `ffcmd` (constructs a `Command` for `ffmpeg` or `ffprobe` with `CREATE_NO_WINDOW` on Windows).
