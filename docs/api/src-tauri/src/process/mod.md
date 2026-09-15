# src-tauri/src/process/mod.rs

MODULE OVERVIEW: child-process plumbing the whole app shares. One submodule today.

**Why it is not under `win/`.** `proc.rs` was `process/proc.rs` until cross-platform Phase 1, Batch B (2026-09-15). Of the 52 `crate::win::` call sites in the crate, 35 were this file (`ffcmd` x22, `tmp_sibling` x5, `generate_once` x4, `preserve_corrupt` x2, `corrupt_sibling` x2, `init_ffmpeg`), spread across `export/`, `edit/`, `settings/`, `ai/` and `encode/`, and it has exactly one `#[cfg(windows)]` line (the `CREATE_NO_WINDOW` creation flag). Treating it as platform code would have pulled half the app into the platform tree; see `docs/cross-platform-architecture.md` section 5.

## proc

Discovers the bundled ffmpeg/ffprobe binaries at startup and exposes a `Command` builder that suppresses console-window flicker on Windows, plus the temp-sibling / corrupt-sibling helpers and `generate_once`, the one-at-a-time lock that stops concurrent ffmpeg passes over the same source. See `proc.md`.
