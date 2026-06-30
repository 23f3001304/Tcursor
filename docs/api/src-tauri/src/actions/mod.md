# src-tauri/src/actions/mod.rs

MODULE OVERVIEW: The `actions` module records every hotkey-driven event that occurs during a recording session. It owns the data model for action kinds and timestamped events, a parser that converts user-configured hotkey strings into armed chord descriptors, and a background polling tracker that detects key presses and typing bursts without installing a kernel hook. The three submodules connect as a pipeline: `model` defines the shared data types, `matcher` parses and matches hotkeys against those types, and `keyboard` runs the poll loop that feeds `ActionEvent` values into `model::ActionLog`. The most important entry points are `matcher::arming_from_settings` (called once at recording start to build the armed table) and `keyboard::KeyboardTracker::start`/`stop` (owns the poll thread for the duration of a session).

## keyboard

Polls Win32 async key state at ~60 Hz on a dedicated background thread to detect hotkey presses and typing bursts without a `WH_KEYBOARD_LL` hook. Key items: `KeyboardTracker` (owns the poll thread and accumulates data), `KeyboardTracker::start` (spawns the thread with an armed chord table), `KeyboardTracker::stop` (joins the thread and returns collected events and typing timestamps).

## matcher

Parses hotkey strings and matches live key events against an armed chord table, emitting typed `ActionEvent` values on press and release. Key items: `KeyChord::parse` (converts `"Ctrl+Alt+Z"` strings to typed chords), `arming_from_settings` (builds the `Vec<Arm>` table from `HotkeySettings`), `ActionMatcher::on_key` (stateful edge-detection matcher used by hook-based sources), `Mods` (modifier snapshot), `Arm` (chord paired with down/up action kinds).

## model

Pure data types for the hotkey event log: action kind enum, timestamped event struct, and JSON-persistable log with save/load helpers. Key items: `ActionKind` (discriminated union covering layout, zoom, spotlight, and video-FX variants), `ActionEvent` (millisecond timestamp plus kind), `ActionLog::save` and `ActionLog::load` (round-trip `actions.json`), `LayoutId` (five layout preset identifiers).
