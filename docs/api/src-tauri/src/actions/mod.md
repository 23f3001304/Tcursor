# src-tauri/src/actions/mod.rs

MODULE OVERVIEW: The `actions` module owns what a hotkey IS - the data model for action kinds and timestamped events, and the parser that turns user-configured hotkey strings into armed chord descriptors. What a hotkey press is DETECTED by is not here: the poll loop, `key_down` and the `TYPING_VKS` table are Win32 virtual-key facts and live in `platform/windows/input/hotkeys.rs` behind `ports::input::HotkeyPort`. The two submodules connect as a pipeline: `model` defines the shared data types and `matcher` parses and matches hotkeys against them. The entry point is `matcher::arming_from_settings`, called once at recording start to build the armed table the port's `hotkeys` factory is handed.

## matcher

Parses hotkey strings and matches live key events against an armed chord table, emitting typed `ActionEvent` values on press and release. Key items: `KeyChord::parse` (converts `"Ctrl+Alt+Z"` strings to typed chords), `arming_from_settings` (builds the `Vec<Arm>` table from `HotkeySettings`), `ActionMatcher::on_key` (stateful edge-detection matcher used by hook-based sources), `Mods` (modifier snapshot), `Arm` (chord paired with down/up action kinds).

## model

Pure data types for the hotkey event log: action kind enum, timestamped event struct, and JSON-persistable log with save/load helpers. Key items: `ActionKind` (discriminated union covering layout, zoom, spotlight, and video-FX variants), `ActionEvent` (millisecond timestamp plus kind), `ActionLog::save` and `ActionLog::load` (round-trip `actions.json`), `LayoutId` (five layout preset identifiers).
