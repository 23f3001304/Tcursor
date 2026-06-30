# src-tauri/src/events/mod.rs

MODULE OVERVIEW: The `events` module records every observable input event during a recording session and persists it for the export pipeline. It is organized in two layers: shared data types and persistence (`model`, `cursortype`, `typing`) sit at the bottom and are consumed by active collectors (`collector`, `tracker`, `cursortracker`) that run on dedicated threads during recording. Mouse events flow from the Windows low-level hook in `tracker` through the deduplication filter in `collector` and land in `EventLog`; cursor shape changes flow from the polling loop in `cursortracker` and land in `CursorTrack`; keystroke timestamps flow into `TypingLog`. All three logs are written to disk by `session::recorder_threads` at stop time and read back by the export and AI-director pipelines.

## model

Core data types and persistence helpers for the mouse-event log. Defines `Button`, `EventKind`, `MouseEvent`, `ScreenInfo`, and the top-level `EventLog` with `save`/`load`. Key items: `MouseEvent` (single timestamped event with kind and screen coordinates), `EventLog` (root of `events.json` with screen geometry and event slice), `EventLog::save`, `EventLog::load`.

## collector

Stateful buffer that accumulates raw mouse events from the hook thread and applies two deduplication filters before storage. `Move` events are throttled by a minimum interval and dropped when coordinates are unchanged; `Down`/`Up` events always pass through. Key items: `EventCollector` (owns the accepted event buffer and filter state), `EventCollector::new` (constructs with configurable throttle interval), `EventCollector::push` (applies filters and appends accepted events), `EventCollector::take` (consumes the collector and returns the event vec).

## tracker

Windows low-level mouse hook running on a dedicated message-loop thread. Translates Win32 `WM_MOUSE*` messages into `MouseEvent` values, timestamps them against the recording clock, and forwards them to an `EventCollector` via a process-global `Mutex`. Key items: `MouseTracker` (RAII handle for the hook thread), `MouseTracker::start` (spawns hook thread, returns immediately), `MouseTracker::stop` (posts `WM_QUIT`, joins thread, returns collected events).

## typing

Persists a privacy-safe keystroke timeline: timestamps only, no key identities. Used by the smart-zoom-hold heuristic to extend a zoom hold when the user keeps typing after clicking. Key items: `TypingLog` (bare `Vec<u32>` of recording-clock millisecond timestamps), `TypingLog::save`, `TypingLog::load` (returns empty log on any error rather than propagating).

## cursortype

Defines the nine recognized OS cursor shapes and the timestamped change-log used to replay the correct sprite per frame during export. Windows Graphics Capture excludes the OS cursor from the video stream, so the shape must be sampled live and stored. Key items: `CursorType` (nine-variant enum matching IDC constants), `CursorTrack` (monotonic log of `(t_ms, CursorType)` pairs), `CursorTrack::type_at` (binary-search lookup returning the active shape at any timestamp), `CursorTrack::save`, `CursorTrack::load`.

## cursortracker

Background thread that polls `GetCursorInfo` at 60 Hz and appends an entry to the shape-change log whenever the cursor type changes. Key items: `CursorTypeTracker` (RAII handle with `Arc<AtomicBool>` stop flag), `CursorTypeTracker::start` (spawns polling thread, builds IDC lookup table once), `CursorTypeTracker::stop` (signals the thread, joins it, returns the shape-change vec).
