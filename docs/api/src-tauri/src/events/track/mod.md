# src-tauri/src/events/track/mod.rs

Submodule overviews for the `track` group.

## tracker

Windows low-level mouse hook running on a dedicated message-loop thread. Translates Win32 `WM_MOUSE*` messages into `MouseEvent` values, timestamps them against the recording clock, and forwards them to an `EventCollector` via a process-global `Mutex`. Key items: `MouseTracker` (RAII handle for the hook thread), `MouseTracker::start` (spawns hook thread, returns immediately), `MouseTracker::stop` (posts `WM_QUIT`, joins thread, returns collected events).

## cursortracker

Background thread that polls `GetCursorInfo` at 60 Hz, appending an entry to the shape-change log whenever the cursor type changes AND capturing each distinct cursor's real bitmap into the project's cursor layer. Key items: `CursorTypeTracker` (RAII handle with `Arc<AtomicBool>` stop flag), `CursorSamples` (what one take produces: the shape log plus the layer builder), `CursorTypeTracker::start` (spawns polling thread, builds IDC lookup table once), `CursorTypeTracker::stop` (signals the thread, joins it, returns both).

## cursorcapture

The one Win32 seam of the cursor layer (Windows-only): `GetIconInfo` + `GetObjectW` + `GetDIBits` turn a live `HCURSOR` into its actual bitmap and hotspot, freeing both GDI bitmaps on every path. Key item: `capture`. Animated cursors capture their first frame only.

## cursorlayer

The captured OS-cursor layer - the real cursor bitmaps plus a `(t_ms, id)` timeline - serialized as `cursor/layer.json` with one PNG per shape. Its presence is also what tells a post-layer recording (video captured clean) apart from a pre-layer one (System baked the cursor into the pixels). Key items: `CursorLayer` (`exists`, `load`, `id_at`), `CursorLayerBuilder` (`add`, `mark`, `save`), `CursorEntry`, `MAX_CURSORS`.

## cursorpixels

Pure conversion of Win32 cursor bitmaps to straight-alpha, top-down RGBA - the mask/BGRA rules split out of `cursorcapture` so they can be unit-tested with no Windows session. Key items: `CapturedCursor`, `color_rgba` (32bpp with real alpha, or the AND-mask fallback for an all-zero alpha channel), `mono_rgba` (the AND/XOR truth table for a NULL-colour cursor).

## cursortype

Defines the nine recognized OS cursor shapes and the timestamped change-log used to replay the correct sprite per frame during export. Windows Graphics Capture excludes the OS cursor from the video stream, so the shape must be sampled live and stored. Key items: `CursorType` (nine-variant enum matching IDC constants), `CursorTrack` (monotonic log of `(t_ms, CursorType)` pairs), `CursorTrack::type_at` (binary-search lookup returning the active shape at any timestamp), `CursorTrack::save`, `CursorTrack::load`.

## typing

Persists a privacy-safe keystroke timeline: timestamps only, no key identities. Used by the smart-zoom-hold heuristic to extend a zoom hold when the user keeps typing after clicking. Key items: `TypingLog` (bare `Vec<u32>` of recording-clock millisecond timestamps), `TypingLog::save`, `TypingLog::load` (returns empty log on any error rather than propagating).
