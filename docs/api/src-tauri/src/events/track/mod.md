# src-tauri/src/events/track/mod.rs

Submodule overviews for the `track` group.

## tracker

Windows low-level mouse hook running on a dedicated message-loop thread. Translates Win32 `WM_MOUSE*` messages into `MouseEvent` values, timestamps them against the recording clock, and forwards them to an `EventCollector` via a process-global `Mutex`. Key items: `MouseTracker` (RAII handle for the hook thread), `MouseTracker::start` (spawns hook thread, returns immediately), `MouseTracker::stop` (posts `WM_QUIT`, joins thread, returns collected events).

## cursortracker

Background thread that polls `GetCursorInfo` at 60 Hz and appends an entry to the shape-change log whenever the cursor type changes. Key items: `CursorTypeTracker` (RAII handle with `Arc<AtomicBool>` stop flag), `CursorTypeTracker::start` (spawns polling thread, builds IDC lookup table once), `CursorTypeTracker::stop` (signals the thread, joins it, returns the shape-change vec).

## cursortype

Defines the nine recognized OS cursor shapes and the timestamped change-log used to replay the correct sprite per frame during export. Windows Graphics Capture excludes the OS cursor from the video stream, so the shape must be sampled live and stored. Key items: `CursorType` (nine-variant enum matching IDC constants), `CursorTrack` (monotonic log of `(t_ms, CursorType)` pairs), `CursorTrack::type_at` (binary-search lookup returning the active shape at any timestamp), `CursorTrack::save`, `CursorTrack::load`.

## typing

Persists a privacy-safe keystroke timeline: timestamps only, no key identities. Used by the smart-zoom-hold heuristic to extend a zoom hold when the user keeps typing after clicking. Key items: `TypingLog` (bare `Vec<u32>` of recording-clock millisecond timestamps), `TypingLog::save`, `TypingLog::load` (returns empty log on any error rather than propagating).
