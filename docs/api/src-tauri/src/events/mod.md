# src-tauri/src/events/mod.rs

MODULE OVERVIEW: The `events` module records every observable input event during a recording session and persists it for the export pipeline. It is organized in two layers: shared data types and persistence (`model`, `cursortype`, `typing`) sit at the bottom and are consumed by active collectors (`collector`, `tracker`, `cursortracker`) that run on dedicated threads during recording. Mouse events flow from the Windows low-level hook in `tracker` through the deduplication filter in `collector` and land in `EventLog`; cursor shape changes flow from the polling loop in `cursortracker` and land in `CursorTrack`; keystroke timestamps flow into `TypingLog`. All three logs are written to disk by `session::recorder_threads` at stop time and read back by the export and AI-director pipelines.

## model

Core data types and persistence helpers for the mouse-event log. Defines `Button`, `EventKind`, `MouseEvent`, `ScreenInfo`, and the top-level `EventLog` with `save`/`load`. Key items: `MouseEvent` (single timestamped event with kind and screen coordinates), `EventLog` (root of `events.json` with screen geometry and event slice), `EventLog::save`, `EventLog::load`.

## remap

The coordinate mapping a mid-take display switch installs on the mouse hook (2026-09-14). A take keeps ONE `ScreenInfo`, so when `session::record::switch_display` moves the capture to another monitor, every later sample is subtracted from that monitor's origin, scaled into the fit rectangle the encoder canvas gives it (`frame_fit::letterbox`) and shifted back into the take's own display's absolute coordinates - integer throughout, and clamped to the canvas. Key items: `Remap`, `Remap::for_switch`, `Remap::apply`.

## collector

Stateful buffer that accumulates raw mouse events from the hook thread and applies two deduplication filters before storage. `Move` events are throttled by a minimum interval and dropped when coordinates are unchanged; `Down`/`Up` events always pass through. Key items: `EventCollector` (owns the accepted event buffer and filter state), `EventCollector::new` (constructs with configurable throttle interval), `EventCollector::push` (applies filters and appends accepted events), `EventCollector::take` (consumes the collector and returns the event vec).

