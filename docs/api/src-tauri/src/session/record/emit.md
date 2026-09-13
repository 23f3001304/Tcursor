# src-tauri/src/session/record/emit.rs

The recording session's two frontend event bridges. Split out of `recorder.rs` (which sits at its line cap) when the level feed was added, so both live together rather than one being wherever there happened to be room.

## emitter

```rust
pub fn emitter(app: &tauri::AppHandle, event: &'static str) -> Notify
```

A `Notify` that forwards its reason to the frontend as `event`. Recording threads hold these instead of an `AppHandle`, so nothing below this file needs to know about Tauri, and the capture pipeline stays constructible in tests.

### Used for

`record-warning` (an audio input that would not open), `record-ended-early` (the OS ended the capture).

## level_emitter

```rust
pub fn level_emitter(app: &tauri::AppHandle, source: &'static str) -> Level
```

A `Level` that forwards each reading to the frontend as `audio-level`, tagged with which capture it came from (`"mic"` / `"system"`).

### Where it runs

From the capture thread's own poll loop, **never** from the audio callback - a realtime thread may not do IPC. The callback's only job is `LevelSlot::push`; see `recorder_threads::poll_until_stopped`.

### Consumed by

`src/hud/hooks/useAudioLevels.ts`, which feeds the HUD's wave meter.
