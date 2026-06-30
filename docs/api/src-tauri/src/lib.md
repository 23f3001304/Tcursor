# src-tauri/src/lib.rs

Tauri application entry point. Declares all top-level modules and wires the Tauri builder with plugins, managed state, IPC handlers, and startup side effects. It also serves as the crate's module index: the `## <module>` sections below define each top-level module and name its most important items.

## run

```rust
pub fn run()
```

Builds and launches the Tauri application. Panics with `"error while running tauri application"` if the event loop exits with an error.

### Inputs

None. All configuration is sourced from `tauri::generate_context!()` (the `tauri.conf.json` baked in at compile time) and from the startup side effects in the `setup` closure.

### Returns

`()`. Does not return in normal operation - the function blocks on the Tauri event loop until the application exits.

### Implementation

1. `tauri::Builder::default()` - start the builder.
2. `.plugin(tauri_plugin_opener::init())` - register the opener plugin for OS-level file and URL opening.
3. `.manage(session::recorder::Recorder::default())` - register the shared `Recorder` state. *Why a single managed instance:* the recorder is stateful (tracks recording lifecycle) and must be accessible from any IPC command handler without passing it explicitly.
4. `.invoke_handler(tauri::generate_handler![...])` - register all IPC command handlers: `list_displays`, `list_audio_inputs`, `start_recording`, `pause_recording`, `resume_recording`, `stop_recording`, `save_webcam`, `export_project`, `get_settings`, `set_settings`, `get_edit`, `apply_edit_op`, `save_edit`, `ai_autoedit`, `preview_frame`, `set_capturable`.
5. `.setup(|app| { ... Ok(()) })` - run startup side effects:
   a. Call `win::proc::init_ffmpeg(app.path().resource_dir().ok())` to locate the bundled ffmpeg/ffprobe and store the directory in `FFMPEG_DIR`. Write the diagnostic log to `%TEMP%/tcursor-ffmpeg.log`. *Why write a log:* any "ffmpeg not available" failure on a user machine is explainable without attaching a debugger.
   b. `std::thread::spawn(encode::ffmpeg_encoder::prewarm)` - warm up the encoder off the main thread. *Why at startup:* audio capture must not stall behind the latency of the first ffmpeg process launch; prewarming ensures the encoder is ready before the user starts recording.
   c. (Windows only, when `CAPTURE_EXCLUDE = true`) Retrieve the main webview window's HWND and call `win::capture_exclusion::set_capture_exclusion(hwnd, true)`. Logs `"capture exclusion applied"` to stdout on success, or a warning to stderr on failure. The compile-time constant `CAPTURE_EXCLUDE` can be set to `false` during design work to allow screenshotting the HUD.
6. `.run(tauri::generate_context!()).expect(...)` - start the event loop.

## ai

The AI Director: local-LLM auto-editing. Serializes the recording's semantic events to a transcript, asks a local model (Ollama) for zooms/trim, clamps the untrusted reply, and applies it to edit.json. This is the trust boundary for model output.

Key items: `commands::ai_autoedit` (the one-shot command), `timeline::serialize` (events -> transcript), `ollama::chat` (HTTP to localhost:11434), `plan::ops_from_json` (parse + clamp -> `EditOp`s), `prompt::system_prompt`.

## domain

Core domain value types and test seams shared across capture, recording, and export. Pure data and trait abstractions with no platform code, so the pipeline can be driven by fakes in tests.

Key items: `time::Timestamp` with `time::Clock` / `SystemClock` / `FakeClock`, `ids::FrameIndex`, `ids::ScreenCoord` / `CanvasCoord`, `capture_source::CaptureSource`.

## capture

Screen frame acquisition. Defines the `FrameSource` abstraction and the Windows Graphics Capture implementation that feeds frames into the recording session.

Key items: `frame_source::FrameSource` (trait + `FakeFrameSource`), `windows_capture::WgcFrameSource` (`for_primary_display`, `next_frame`, `drain_latest`), `frame::Frame`.

## encode

Video encoding. Defines the `FrameSink` abstraction and the ffmpeg-backed sink that encodes frames to mp4 (NVENC when available).

Key items: `frame_sink::FrameSink` (trait + `FakeFrameSink`), `ffmpeg_encoder::FfmpegFrameSink` (`new`, `new_hq`, `push`, `finish`), `ffmpeg_encoder::prewarm`.

## audio

Audio capture and WAV output: microphone (cpal) and system loopback (WASAPI), each timestamped for A/V sync.

Key items: `audio_source::AudioSource`, `cpal_mic::CpalMic` (`open`, `default_input`), `system_audio::SystemAudio::loopback`, `wav_writer::WavWriter`.

## session

Recording orchestration and lifecycle: the Tauri `Recorder` state, the start/pause/resume/stop commands, the per-frame capture loop, project paths, and the A/V sync log. This is the heart of "record".

Key items: `recorder::Recorder` with `start_recording` / `pause_recording` / `resume_recording` / `stop_recording`, `recording_session::RecordingSession` (`run`, `run_paced`), `paths::ProjectPaths`, `sync::SyncLog`, `pacing` (CFR game mode), `recorder_threads::save_inputs`.

## win

Windows platform glue used across recording and export: display refresh query, bundled-ffmpeg resolution, HUD capture exclusion, and OS theme detection.

Key items: `display::primary_refresh_hz`, `proc::init_ffmpeg` / `ffcmd` / `FFMPEG_DIR`, `capture_exclusion::exclude_from_capture`, `theme::os_prefers_dark` / `resolve_dark`.

## commands

Miscellaneous Tauri IPC commands not owned by a feature module: device enumeration, webcam save, export kickoff, and settings get/set.

Key items: `list_displays`, `list_audio_inputs`, `save_webcam`, `export_project`, `get_settings`, `set_settings` (plus the `DisplayInfo` / `AudioInfo` DTOs).

## events

Input event capture during recording: the Win32 mouse hook, cursor-shape tracking, and typing timestamps, plus their serialized logs. This is the semantic "moat" data the auto-zoom and AI Director consume.

Key items: `tracker::MouseTracker`, `cursortracker::CursorTypeTracker`, `model::EventLog` / `MouseEvent`, `cursortype::CursorTrack`, `typing::TypingLog`, `collector::EventCollector`.

## actions

Hotkey-driven actions: a low-level keyboard hook plus chord parsing and matching that turn key combos (and typing) into `ActionEvent`s (e.g. spotlight hold) recorded for export.

Key items: `keyboard::KeyboardTracker`, `matcher::KeyChord` / `ActionMatcher` / `arming_from_settings`, `model::ActionEvent` / `ActionLog`.

## export

The render pipeline: turns a raw recording plus edit.json into the final mp4 (decode -> compose scene -> draw overlays -> encode -> mux). The largest module; see `export/mod.rs` for the full submodule map.

Key items: `run::run_export` (entry), `exporter::export`, `autozoom::generate`, `scene` / `compositor` / `gpu_compositor`, `coordmap`, `camera::CameraSim`, and the draw and fx layers.

## settings

Persisted recorder/export configuration: the `Settings` model, its JSON store, and appearance helpers. Snapshotted at record time so an export reproduces the exact configuration.

Key items: `model::Settings` (and all config structs/enums, `ZoomSettings::to_zoom_config`), `store::load` / `save`, `appearance` (layout and overlay resolution).

## edit

The edit.json data layer (M3 editor foundation): the single editable `EditDoc`, the Edit API that mutates it, seeding from a recording, and the Tauri commands that GUI and AI drivers call.

Key items: `model::EditDoc` / `Zoom` / `EditOp`, `api::apply` / `metrics`, `seed::load_or_seed`, `commands::get_edit` / `apply_edit_op` / `save_edit`.
