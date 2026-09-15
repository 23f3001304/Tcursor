# src-tauri/src/asr/whisper.rs

The whisper.cpp run itself, and the ONLY file in the tree that knows the `whisper-rs` crate exists. It returns RAW tokens; `words::words_from_tokens` (pure, tested) turns those into words.

That seam is load-bearing twice over: it is what lets every other file in `asr/` be tested with no model on disk, and it is where a `whisper-cli` fallback (plan T0's declined branch) would plug in with nothing else in the milestone changing.

**GPU backend (owner ruling 2026-09-15).** `whisper-rs` 0.16 is built with its `vulkan` feature, so inference runs on the GPU through whisper.cpp's Vulkan backend: one build serves NVIDIA, AMD and Intel, the app ships nothing extra (every current Windows driver carries Vulkan), and `WhisperContextParameters::default()` turns `use_gpu` on whenever a GPU feature is compiled in. On the owner's machine the log reads `ggml_vulkan: Found 2 Vulkan devices: 0 = NVIDIA GeForce RTX 5080 ... matrix cores: NV_coopmat2`; device 0 is used. CUDA is the opt-in cargo feature `asr-cuda` (`whisper-rs/cuda`): faster on NVIDIA but it needs the CUDA toolkit to build and its runtime DLLs beside the exe, so it is not the shipped default. Plan T0's CPU-only choice was a build-convenience call the owner overruled ("why leave acceleration").

**Build prerequisites.** `whisper-rs-sys` vendors whisper.cpp and drives cmake over it. Building needs CMake 3.14+ on PATH, the MSVC C++ workload, an LLVM whose libclang bindgen can find (`LIBCLANG_PATH` if it is not on a default search path), and the Vulkan SDK (`winget install KhronosGroup.VulkanSDK`; it sets `VULKAN_SDK`, and its `glslc` compiles the ~300 shader variants at build time, about 3.5 minutes on a warm machine). One Windows trap: whisper.cpp's shader generator is a nested CMake project whose object paths pass CMake's 250-character limit from the repo's default `src-tauri/target`, so `src-tauri/.cargo/config.toml` points the crate at `C:/tct`; that file must hold nothing else - `src-tauri/Cargo.toml`'s own comment records why the plan's `WHISPER_DONT_GENERATE_BINDINGS=1` shortcut is NOT used here (the crate's pre-generated bindings were produced on Linux and their glibc `size_of` assertions are false against the MSVC CRT).

## AsrParams

```rust
pub struct AsrParams { pub model_path: PathBuf, pub language: String, pub threads: i32, pub vad_model: Option<PathBuf> }
```

What one transcription needs. `language` is a whisper language code, or `"auto"` to let the model detect it - which only a multilingual model can do, a constraint `models::resolve_model` enforces with a message before anything gets this far. `vad_model` is the Silero VAD file (`models::VAD_MODEL`, `models::vad_path`) when it is on disk: with it, whisper.cpp's own voice activity detection runs first and only the speech spans are decoded, which removes the hallucinated words silence produces and skips the silent stretches instead of decoding them. `None` (offline, or the small download failed) decodes everything and leans on `drop_silent` alone. `transcribe_project` fetches the file best-effort before every run, so on a machine that has been online once it is always `Some`.

## default_threads

```rust
pub fn default_threads() -> i32
```

Every core the machine has, capped at 8. Past 8, whisper.cpp's own scaling flattens while the editor it is sharing the machine with starts to stutter, so the cap buys responsiveness at almost no speed cost. Falls back to 4 when the core count is unavailable.

## transcribe

```rust
pub fn transcribe(pcm: &[f32], p: &AsrParams, on_pct: &dyn Fn(u32), cancel: &dyn Fn() -> bool) -> Result<Vec<RawToken>, String>
```

Runs greedy decoding (`best_of: 1`) over 16 kHz mono f32 PCM with token timestamps on and every whisper.cpp print off, and returns its raw tokens with the ones over near-silent audio removed (`drop_silent`).

One run per process at a time: the function holds a static `Mutex` (`ONE_AT_A_TIME`) for its whole body, because ggml's Vulkan backend keeps global state and two whisper contexts initialised concurrently die with `STATUS_ACCESS_VIOLATION` (the two model-gated tests found it the moment the GPU backend landed; serially they pass). `transcribe_project`'s in-flight set already keeps the app to one transcription, so the lock is the guarantee behind that policy rather than a second policy.

`on_pct` is called with 0..100 as the decode advances. `cancel` is polled on every step and aborts promptly when it returns `true` - `transcribe_project` wires it to the folder's presence in its in-flight set, so removing the folder is what stops a decode.

**The callbacks are raw on purpose (2026-09-15).** The first real transcription crashed the app with `STATUS_ACCESS_VIOLATION` right after `whisper_init_state`. Cause: whisper-rs 0.16.0's `set_abort_callback_safe` stores the closure as a `Box<Box<dyn FnMut() -> bool>>` but instantiates its C trampoline with the UNBOXED closure type, so the first abort poll inside `whisper_full` reinterprets the box's fat pointer as the closure and jumps through a garbage function pointer (the progress setter next to it gets this right, which is why only the abort path died). So this file sets `set_progress_callback` / `set_abort_callback` with its own two `extern "C"` trampolines and one `user_data`: a private `Hooks` struct holding the two borrowed closures, which lives on `transcribe`'s own frame and therefore outlives `state.full`, the only thing that can call the trampolines. No `'static` bound, no `transmute`, no leaked boxes.

## SILENCE_RMS

```rust
pub const SILENCE_RMS: f32 = 0.001;
```

The RMS floor (about -60 dBFS) under which a token's audio window counts as silence. Whisper base.en reliably hallucinates a word on pure silence (one second of zeros decodes to "you"), and a caption for a word nobody said is worse than no caption.

## drop_silent

```rust
pub fn drop_silent(tokens: Vec<RawToken>, pcm: &[f32]) -> Vec<RawToken>
```

Keeps a token when the PCM under its `[t0_cs, t1_cs)` window (centiseconds, 160 samples each at 16 kHz) has RMS at or above `SILENCE_RMS`. A zero-width or inverted window is kept as is, since there is nothing to measure and `collect_tokens` has already given it a share of its segment. Pure, so it is tested without a model.

### collect_tokens

Private. Walks every segment's tokens, taking each token's own `t0`/`t1` when they are ordered. A token whose bounds are missing or inverted (whisper.cpp emits `t1 == t0` on some pieces) falls back to an even share of its SEGMENT's span, so a word built from it still lands roughly where it was spoken instead of at zero - which is what a naive fallback would do, and it would put a caption at the start of the video for something said a minute in.

### Behaviors

The two decode tests are MODEL-GATED (plan ADDED-10): they check `models::model_path("base.en").exists()` and, when it is absent, print `SKIP <name>: ggml-base.en.bin not installed (run the Captions panel's download once)` and return. No test in this tree ever downloads a model, so a clean checkout is green and "it passed" never quietly means "it was skipped". Once the model is installed they run real inference, which is what caught the callback crash.

- `one_second_of_silence_transcribes_to_no_usable_tokens` - silence is not an error, and produces no alphanumeric words (the model's "you" is dropped by the silence gate).
- `a_cancel_returns_promptly_instead_of_running_to_completion` - 30 s of PCM with `cancel` always true returns in under 20 s.
- `a_token_over_silence_is_dropped_and_one_over_sound_is_kept` - needs no model; a token over zeros goes, one over a sine stays, a zero-width token stays.
- `the_thread_count_is_capped_so_a_transcription_never_takes_the_whole_machine` - needs no model; pins `default_threads` into 1..=8.
