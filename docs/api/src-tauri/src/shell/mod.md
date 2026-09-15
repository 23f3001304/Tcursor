# src-tauri/src/shell/mod.rs

MODULE OVERVIEW: Tauri-only chrome around the app - code that talks to the window manager through Tauri's own API, and the one place allowed to turn a Tauri window into something the ports can take. Created in cross-platform Phase 1, Batch B (2026-09-15) for `brand_icon`, which had lived under `win/` behind gates it did not need; `window` joined it in Batch C3; see `docs/cross-platform-architecture.md` section 5.

## brand_icon

The REC-lit app icon while recording and the taskbar progress bar during export. Key items: `set_recording`, `set_export_progress`. Both swallow every failure. See `brand_icon.md`.

## window

Resolves a `tauri::WebviewWindow` to the opaque `WindowHandle` the system port takes, so neither `ports/` nor `platform/` ever names a Tauri type. Key items: `handle` (the raw HWND as an `isize` on Windows, `None` elsewhere; the window is always a parameter, never looked up, because Studio is a second window). See `window.md`.
