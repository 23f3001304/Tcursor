# src-tauri/src/session/record/emit.rs

The recording session's frontend event bridges, and the bundle a take is handed them in. Split out of `recorder.rs` (which sits at its line cap) when the level feed was added, so both live together rather than one being wherever there happened to be room.

## TakeHooks

```rust
pub struct TakeHooks {
    pub warn: Notify,
    pub ended: Notify,
    pub mic_level: Option<Level>,
    pub system_level: Option<Level>,
}
```

Everything one take reports OUT through, in one value: the degraded-but-running warning, the OS-ended-the-capture notice, and the two live level feeds.

*Why a bundle rather than four arguments.* `start_take` builds all four from the same `AppHandle` and hands three of them to three different threads. Bundling them is what let the `AppHandle` leave that function's signature entirely, which is what makes a take startable from a test.

- `warn` / `ended` - `Notify`, cloned into the mic thread, the system thread and the capture request. `ended` reaches `platform::windows::capture` as `CaptureRequest::ended`.
- `mic_level` / `system_level` - `Option<Level>`, `None` meaning "nobody is watching the meter". The option is what `TakeHooks::silent` uses; in the app both are always `Some`.

## TakeHooks::from_app

```rust
pub fn from_app(app: &tauri::AppHandle) -> Self
```

The four real bridges: `record-warning`, `record-ended-early`, and `audio-level` tagged `"mic"` and `"system"`. The ONE place a take's reporting is attached to Tauri.

## TakeHooks::silent

```rust
#[cfg(test)]
pub fn silent() -> Self
```

Hooks that discard everything. `#[cfg(test)]` so it cannot be reached from a shipping build, where a take that reported nothing would be a take whose warnings the user never saw. Used by `platform/mock/cycle_tests.rs`.

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
