# src-tauri/src/events/track/mod.rs

Submodule overviews for the `track` group.

## cursortracker

`CursorSamples` - what one take's cursor polling produces, the shape log plus the captured layer. It stays here because it is the return type of `ports::input::CursorShapePort::stop` and nothing about it is a Windows fact. Batch D deleted `tracker.rs` and the `CursorTypeTracker` alias with it: the recorder holds the three input ports as trait objects, so nothing outside `platform/windows/input/` names an adapter type.

## cursorlayer

The captured OS-cursor layer - the real cursor bitmaps plus a `(t_ms, id)` timeline - serialized as `cursor/layer.json` with one PNG per shape. Its presence is also what tells a post-layer recording (video captured clean) apart from a pre-layer one (System baked the cursor into the pixels). Key items: `CursorLayer` (`exists`, `load`, `id_at`), `CursorLayerBuilder` (`add`, `mark`, `save`), `CursorEntry`, `MAX_CURSORS`.

## cursorpixels

Pure conversion of Win32 cursor bitmaps to straight-alpha, top-down RGBA - the mask/BGRA rules split out of the GDI reads (now `platform/windows/input/bitmap.rs`) so they can be unit-tested with no Windows session. Key items: `CapturedCursor`, `color_rgba` (32bpp with real alpha, or the AND-mask fallback for an all-zero alpha channel), `mono_rgba` (the AND/XOR truth table for a NULL-colour cursor).

## steady

The shape-hold filter (`SHAPE_HOLD_MS`, `steady`) both cursor tracks are read through, so a pointer crossing text does not jitter the drawn cursor. See `steady.md`.

## cursortype

Defines the nine recognized OS cursor shapes and the timestamped change-log used to replay the correct sprite per frame during export. Windows Graphics Capture excludes the OS cursor from the video stream, so the shape must be sampled live and stored. Key items: `CursorType` (nine-variant enum matching IDC constants), `CursorTrack` (monotonic log of `(t_ms, CursorType)` pairs), `CursorTrack::type_at` (binary-search lookup returning the active shape at any timestamp), `CursorTrack::save`, `CursorTrack::load`.

## typing

Persists a privacy-safe keystroke timeline: timestamps only, no key identities. Used by the smart-zoom-hold heuristic to extend a zoom hold when the user keeps typing after clicking. Key items: `TypingLog` (bare `Vec<u32>` of recording-clock millisecond timestamps), `TypingLog::save`, `TypingLog::load` (returns empty log on any error rather than propagating).
