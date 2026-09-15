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
2. `.plugin(tauri_plugin_opener::init())` - register the opener plugin for OS-level file and URL opening. `.plugin(tauri_plugin_dialog::init())` - register the dialog plugin for native file/save dialogs.
3. `.manage(session::record::recorder::Recorder::default())` - register the shared `Recorder` state. *Why a single managed instance:* the recorder is stateful (tracks recording lifecycle) and must be accessible from any IPC command handler without passing it explicitly. `.manage(export::preview::PreviewSession::default())` - register the warm preview-renderer cache (`PreviewSession`) so the preview/editor commands share one `FrameRenderer` instead of rebuilding it per call.
4. `.on_window_event(|window, event| { ... })` - registered for all windows (there is only "main"). On `WindowEvent::CloseRequested`, checks `Recorder::is_busy()`; if a take is recording or its stop is still finalizing, calls `api.prevent_close()` and (once per close attempt, guarded by the module-level `CLOSING` static) spawns `session::record::close_guard::finish_and_close` to finalize the take and then close the window itself. *Why here, not left to the frontend alone:* this is the R6 fix for the Critical finding that closing the HUD mid-recording silently destroyed the take - it must hold even if the renderer is hung or the window is closed via the OS chrome / Alt+F4, neither of which run any HUD JS at all. See `close_guard.md`.
5. `.invoke_handler(tauri::generate_handler![...])` - register all IPC command handlers, including (among many others) `start_recording`/`stop_recording`, `session::record::switch_display::switch_display` (changing the recorded display mid-take), `export_project`, `get_settings`/`set_settings`, the `edit`/`ai`/`export::preview`/`export::cursor` command groups, and `session::project::commands::open_project` / `list_recent_projects` / `get_launch_project`.
6. `.setup(|app| { ... Ok(()) })` - run startup side effects:
   a. `app.manage(Arc::new(platform::current()))` - build the ONE adapter bundle for this target and register it as managed state. *Why first, before anything else in `setup`:* the capture exclusion in step (d) is the first thing in the process that reaches an OS API through a port, and every command that resolves `State<'_, Arc<Platform>>` can fire the moment the webview loads. *Why `Arc` and not the bundle itself:* `run_export` and `with_warm_app` hand a port down into work that outlives the command, so the bundle has to be shareable as a handle. See `platform/mod.md`.
   b. `app.manage(session::project::commands::LaunchProject(session::project::commands::launch_project_from_argv(std::env::args())))` - resolve the cold-start file-association argv (a `.tcursor` path from a Windows double-click) into a project folder, if any, and register it as managed state read once by the frontend via `get_launch_project`. *Why here:* cheap and side-effect-free; must run before the frontend's first invoke.
   b. Call `process::proc::init_ffmpeg(app.path().resource_dir().ok())` to locate the bundled ffmpeg/ffprobe and store the directory in `FFMPEG_DIR`. Write the diagnostic log to `%TEMP%/tcursor-ffmpeg.log`. *Why write a log:* any "ffmpeg not available" failure on a user machine is explainable without attaching a debugger.
   c. `std::thread::spawn(encode::ffmpeg_encoder::prewarm)` - warm up the encoder off the main thread. *Why at startup:* audio capture must not stall behind the latency of the first ffmpeg process launch; prewarming ensures the encoder is ready before the user starts recording. Then `std::thread::spawn(export::preview::bg_thumbs::prewarm)` - the background picker's 53 thumbnails, loaded from the cache dir or rendered once and cached, so the editor never asks a cold cache (see `bg_thumbs.md`).
   d. (When `CAPTURE_EXCLUDE = true`) Resolve the main webview window with `shell::window::handle(&win)` and call `platform.system.exclude_from_capture(handle, true)` on the bundle from step (a). Logs `"capture exclusion applied"` to stdout when the affinity reads back as asked for, or a warning to stderr when it does not. The compile-time constant `CAPTURE_EXCLUDE` can be set to `false` during design work to allow screenshotting the HUD. *Why the window is resolved by a helper rather than inline:* the helper takes the window as a parameter, so Studio's launcher window gets the same treatment without a second copy of this code; see `shell/window.md`.
7. `.run(tauri::generate_context!()).expect(...)` - start the event loop.

## ai

The AI Director: local-LLM auto-editing. Serializes the recording's semantic events to a transcript, samples frames of the proxy at those events, asks a local model (Ollama) what to change, and validates the untrusted reply into `EditOp`s that already exist. This is the trust boundary for model output; nothing here writes edit.json.

Key items: `commands::ai_propose` (the propose pass - returns reviewable proposals without applying them; the one-shot `ai_autoedit` command was removed as dead command-surface, sweep-2 Task 7g), `plan::transcript::serialize` (events -> transcript), `frames::sample_times` / `frames::jpegs_at` (which moments get a frame, and the JPEG for each), `llm::vision::has_vision` (can this model see), `llm::ollama::chat_with_images` (HTTP to localhost:11434), `plan::mapping::proposals_from_json` (parse + validate -> proposals), `llm::prompt::system_prompt`.

## asr

Everything speech (M5 captions): the table of Whisper GGML models the app knows about, a resumable and sha256-checked download of one into `<config dir>/TCursor/models/whisper/`, and the IPC the Captions panel drives it with. Nothing outside this module knows what a model file is or where it lives. Transcription itself lands here in later tasks of the milestone.

Key items: `models::MODELS` / `find` / `model_path` / `is_installed` / `resolve_model` (which model a `CaptionStyle` actually means, or a message saying why not), `download::download_model` / `resume_from` / `verify_or_remove`, `sha256::sha256_file` (hand-rolled, no crate), `commands::whisper_models` / `download_whisper_model` / `transcribe_project`, and the transcription pass itself - `audio::{pick_source, decode_16k_mono, source_shift_ms, shift_words}`, `words::words_from_tokens`, `group::group_words`, `whisper::transcribe`.

## domain

Core domain value types and test seams shared across capture, recording, and export. Pure data and trait abstractions with no platform code, so the pipeline can be driven by fakes in tests.

Key items: `time::Timestamp` with `time::Clock` / `SystemClock`. (`ids`, `capture_source` and `FakeClock` were deleted as consumer-less in the cleanup of 2026-09-15.)

## ports

The operating-system boundary as traits and plain data: six ports across four files, with no Windows, macOS or Linux type anywhere under them. Every signature is shaped by a bug the recorder already fixed, and the doc comments say which.

Key items: `capture::TargetId` / `CaptureRequest` / `CaptureGeometry` / `FirstFrameSize`, `capture::VideoSink` (`switch` / `stop` / `supports_switch`) and `capture::CapturePort` (`list_targets` / `bounds` / `start`), `input::PointerPort` / `HotkeyPort` / `CursorShapePort` / `InputPort`, `system::WindowHandle` / `SystemPort`, `audio::SystemAudioPort`.

## platform

The composition root: one bundle of adapters satisfying the `ports` traits, with the crate's ONE platform `#[cfg]` inside `current()`. The Windows adapters forward to the free functions and tracker types the recorder already calls by name, so the seam exists without anything having moved.

Key items: `Platform` (capture, input, system, audio), `current`, `windows::Win32Capture` / `Win32Sink`, `windows::Win32Input` / `Win32Pointer` / `Win32Hotkeys` / `Win32CursorShapes`, `windows::Win32System`, `windows::Win32SystemAudio`.

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

Key items: `record::recorder::Recorder` with `start_recording` / `pause_recording` / `resume_recording` / `stop_recording` / `is_recording` / `is_busy` (the latter two read by `lib.rs`'s `CloseRequested` guard and `record::close_guard::finish_and_close`), `record::recording_session::RecordingSession` (`run`, `run_paced`), `paths::ProjectPaths`, `sync::SyncLog`, `pacing` (CFR game mode), `record::recorder_threads::save_inputs`, `project` (the `.tcursor` manifest, recents list, and `open_project`/file-association commands).

## process

Child-process plumbing shared by the whole app: `process::proc` (ffmpeg/ffprobe discovery, `ffcmd`, temp siblings, `generate_once`). Moved out of `win/` in cross-platform Phase 1, Batch B (2026-09-15). See `process/mod.md`.

## shell

Tauri-only chrome: `shell::brand_icon` (the REC icon, the taskbar progress bar). Moved out of `win/` in the same batch, its `#[cfg(windows)]` gates removed. See `shell/mod.md`.

## commands

Miscellaneous Tauri IPC commands not owned by a feature module: device enumeration, webcam save, export kickoff, and settings get/set.

Key items: `list_displays`, `list_audio_inputs`, `append_webcam`, `export_project`, `get_settings`, `set_settings` (plus the `DisplayInfo` / `AudioInfo` DTOs).

## events

Input event capture during recording: the Win32 mouse hook, cursor-shape tracking, and typing timestamps, plus their serialized logs. This is the semantic "moat" data the auto-zoom and AI Director consume.

Key items: `tracker::MouseTracker`, `cursortracker::CursorTypeTracker`, `model::EventLog` / `MouseEvent`, `cursortype::CursorTrack`, `typing::TypingLog`, `collector::EventCollector`, `remap::Remap` (the coordinate mapping a mid-take display switch installs on the hook, so the log keeps one `ScreenInfo`).

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
