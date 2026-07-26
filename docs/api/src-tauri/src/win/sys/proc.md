# src-tauri/src/win/sys/proc.rs

Locates the bundled `ffmpeg`/`ffprobe` binaries at startup and exposes a `Command` builder that suppresses Windows console-window flicker. All higher-level encode and mux code calls `ffcmd` rather than `Command::new("ffmpeg")` directly so the binary resolution is transparent.

## FFMPEG_DIR

```rust
static FFMPEG_DIR: OnceLock<PathBuf>
```

Process-global directory containing the bundled ffmpeg and ffprobe executables. Written exactly once at startup by `set_ffmpeg_dir`. When unset (e.g. during `cargo test` or dev runs), `resolve` returns the bare program name and lets `PATH` handle lookup.

*Why `OnceLock`:* the directory is discovered once and must never change mid-process; `OnceLock` enforces that contract without a mutex.

## tmp_sibling

```rust
pub fn tmp_sibling(out: &Path) -> PathBuf
```

A unique temporary path in the same directory as `out` (so a `rename` into place is atomic on one volume) with `out`'s original extension preserved (so ffmpeg still infers the container). A producer writes the full output to this temp, then `std::fs::rename`s it onto `out`; a concurrent reader of `out` therefore never observes a half-written file.

### Inputs

- `out: &Path` - the final output path the temp will be renamed onto.

### Returns

`PathBuf` - `<dir>/.part-<pid>-<seq>-<name>`, unique per call via a process-global `AtomicU64` counter so two racing writers get distinct temps (the later `rename` wins, atomically replacing any existing output).

### Used by

- `src-tauri/src/export/preview/preview_track.rs` (`ensure_proxy`) and `src-tauri/src/export/preview/thumbs.rs` (`ensure_waveform`, `ensure_preview_audio`) - so an editor opening while the post-record `preprocess_project` pass is still running never loads a partial proxy/waveform/audio file.

## generate_once

```rust
pub fn generate_once<F: FnOnce() -> Result<(), String>>(out: &Path, gen: F) -> Result<(), String>
```

Serializes and de-duplicates editor-media generation across concurrent callers. Runs `gen` under one process-global `Mutex`, and skips it entirely when `out` already exists.

### Inputs

- `out: &Path` - the final output whose existence means "already generated". Checked *inside* the lock, so a caller that waited behind another producer sees the freshly-created file and skips its own pass.
- `gen: FnOnce() -> Result<(), String>` - the actual generation (the ffmpeg pass + atomic rename). Only invoked when `out` is still missing.

### Returns

`Result<(), String>` - `Ok(())` if the output exists (already or after `gen`), else `gen`'s error.

### Why

Right after recording stops, the HUD's `preprocess_project` pass and (for a legacy/un-preprocessed project, or a non-default proxy quality) the editor's own lazy `ensure_*` can both fire for the same proxy/thumbnails/waveforms/preview-audio. Without this they'd launch a storm of concurrent ffmpeg passes over the same 4K source - each ffmpeg itself multi-threaded - oversubscribing every core exactly when the editor opens (the "editor lags while the preview loads" symptom `preprocess_project` exists to eliminate for the common case). The global lock caps it at one pass at a time; the existence check means the second caller for a file reuses the first's result instead of transcoding again. `gen` stays idempotent (it still writes via `tmp_sibling` + atomic rename).

### Used by

- `src-tauri/src/export/preview/preview_track.rs` (`ensure_proxy`) and `src-tauri/src/export/preview/thumbs.rs` (`ensure_thumbs`, `ensure_waveform`, `ensure_preview_audio`) - every editor-media generator wraps its ffmpeg pass in this.

## set_ffmpeg_dir

```rust
pub fn set_ffmpeg_dir(dir: PathBuf)
```

Stores `dir` into `FFMPEG_DIR`. Only the first call has any effect; subsequent calls are silently ignored by `OnceLock::set`.

### Inputs

- `dir: PathBuf` - the directory that contains `ffmpeg.exe` (or `ffmpeg` on Unix). *Why consumed rather than borrowed:* `OnceLock::set` takes ownership; there is no reason to clone a `PathBuf` that will never be used again after this call.

### Returns

`()`. Errors from `OnceLock::set` (i.e. a second call) are discarded with `let _ = ...`.

### Implementation

1. `let _ = FFMPEG_DIR.set(dir)` - attempt to write; silently discard the `Err(dir)` returned on a second call.

### Used by

- `src-tauri/src/win/sys/proc.rs` (`init_ffmpeg`) - called once with the winning candidate directory

## choose_dir

```rust
pub fn choose_dir<'a>(cands: &'a [PathBuf], name: &str) -> Option<&'a PathBuf>
```

Pure helper that returns the first directory in `cands` that contains a file named `name`.

### Inputs

- `cands: &'a [PathBuf]` - candidate directories in priority order (most-preferred first). *Why a slice:* `ffmpeg_candidates` builds a `Vec`; a slice reference avoids an allocation.
- `name: &str` - file name to probe (e.g. `"ffmpeg.exe"`). *Why a string rather than a `Path`:* the name is built with `EXE_SUFFIX` at the call site; passing it already formatted keeps this function generic.

### Returns

`Option<&'a PathBuf>` - a reference to the first element of `cands` for which `d.join(name).exists()` is true, or `None` if no candidate matches.

### Implementation

1. `cands.iter().find(|d| d.join(name).exists())` - linear scan, short-circuits on first hit.

### Behaviors

- `choose_dir_picks_first_with_the_binary` - creates temp directories `a` (no binary) and `b` (has `ffmpeg.exe`); asserts `choose_dir([a, b], "ffmpeg.exe") == Some(&b)` and `choose_dir([a, b], "nope.exe") == None`.

## ffmpeg_candidates

```rust
pub fn ffmpeg_candidates(resource_dir: Option<&Path>) -> Vec<PathBuf>
```

Builds the priority-ordered list of directories to search for the bundled ffmpeg binary.

### Inputs

- `resource_dir: Option<&Path>` - Tauri's resolved resource directory, if available. *Why optional:* on installed NSIS builds `resource_dir()` may fail to resolve; the exe-derived candidates cover that case.

### Returns

`Vec<PathBuf>` with up to four entries in the following order:

1. `<current_exe_dir>/resources` - the NSIS installer places binaries here
2. `<current_exe_dir>` - bare exe directory (fallback if `resources/` sub-directory is absent)
3. `<resource_dir>/resources` - Tauri resource layout
4. `resource_dir` itself

If `current_exe()` fails, entries 1 and 2 are omitted. If `resource_dir` is `None`, entries 3 and 4 are omitted. The returned `Vec` may be empty but is never `None`.

### Implementation

1. `std::env::current_exe()` - get the running executable path; on success, take `.parent()` and push `parent/resources` then `parent` itself.
2. If `resource_dir` is `Some(res)`, push `res/resources` then `res`.

### Behaviors

- `candidates_lead_with_exe_dir_then_resource_dir` - with `resource_dir = Some("C:/res")`, asserts the last two entries are `"C:/res/resources"` and `"C:/res"`, confirming exe-derived entries lead.

## init_ffmpeg

```rust
pub fn init_ffmpeg(resource_dir: Option<PathBuf>) -> String
```

Orchestrates startup binary discovery and returns a diagnostic log string.

### Inputs

- `resource_dir: Option<PathBuf>` - Tauri's resource directory, passed by `lib.rs` from `app.path().resource_dir()`. *Why `Option<PathBuf>` rather than `Option<&Path>`:* Tauri returns an owned `PathBuf`; taking ownership avoids a clone at the call site.

### Returns

`String` - a multi-line diagnostic that lists: the running exe path; the `resource_dir` argument; all candidate paths tried; the chosen directory; and the fully resolved ffmpeg path that `resolve("ffmpeg")` would return. The caller (`lib.rs`) writes this string to `%TEMP%/tcursor-ffmpeg.log` so any "ffmpeg not available" failure is explainable without attaching a debugger.

### Implementation

1. Build `name = format!("ffmpeg{}", EXE_SUFFIX)` to get the platform-correct binary name.
2. Call `ffmpeg_candidates(resource_dir.as_deref())` to get the ordered candidate list.
3. Call `choose_dir(&cands, &name)` to find the winner.
4. If a winner was found, call `set_ffmpeg_dir(dir.clone())`.
5. Format and return the diagnostic string with all discovery details.

### Used by

- `src-tauri/src/lib.rs` (`run` / `setup` closure) - called once at startup; its return value is written to `%TEMP%/tcursor-ffmpeg.log`

## ffcmd

```rust
pub fn ffcmd(program: &str) -> Command
```

Creates a `std::process::Command` for an ffmpeg-family tool, resolving the binary to the bundled copy and suppressing Windows console-window flicker.

### Inputs

- `program: &str` - tool name without extension: `"ffmpeg"` or `"ffprobe"`. *Why a name rather than a full path:* the caller should not need to know whether the binary is bundled or on PATH; `resolve` encapsulates that decision.

### Returns

`Command` pre-configured with the resolved binary path and, on Windows, the `CREATE_NO_WINDOW` creation flag (`0x0800_0000`). The caller adds arguments and spawns normally.

### Implementation

1. Call the private `resolve(program)`: if `FFMPEG_DIR` is set and the binary exists there, returns the full path; otherwise returns `PathBuf::from(program)` (PATH lookup). *Why check existence:* `FFMPEG_DIR` could be set to a valid directory that is missing one of the two binaries (e.g. a stripped bundle).
2. `Command::new(resolved_path)`.
3. (Windows only, via the private `ffcmd_prio` helper) `c.creation_flags(CREATE_NO_WINDOW | <priority>)` through the `CommandExt` trait - `ffcmd` passes priority `0`. *Why the no-window flag:* without it, each ffmpeg invocation momentarily flashes a black console window on the user's desktop.

### Used by

- `src-tauri/src/encode/ffmpeg_encoder.rs` - creates the encoding `Command` for every recording segment and for the `prewarm` probe
- `src-tauri/src/export/pipeline/audio_mux.rs` - runs ffmpeg for audio track muxing during export
- `src-tauri/src/export/pipeline/ffio.rs` - runs ffprobe (duration probe, format probe) and ffmpeg (remux, concat) throughout the export pipeline

## ffcmd_bg

```rust
pub fn ffcmd_bg(program: &str) -> Command
```

Identical to `ffcmd` but adds `BELOW_NORMAL_PRIORITY_CLASS` (`0x0000_4000`) to the Windows creation flags, so the spawned ffmpeg runs at below-normal process priority.

### Why

A single editor-media transcode (`ensure_proxy`, thumbnails, waveforms) is itself multi-threaded and grabs every core. At normal priority that starves the normal-priority WebView - the editor freezes and can't even decode the raw preview (a blank frame) until the pass finishes. Below-normal lets the OS preempt ffmpeg for the UI, so the editor stays responsive while media loads in the background. This is only for background generation (the post-record `prewarm` + the editor's lazy `ensure_*`); the user-initiated **export** keeps `ffcmd` (full speed) because the user is actively waiting on it.

### Used by

- `src-tauri/src/export/preview/preview_track.rs` (`ensure_proxy`) and `src-tauri/src/export/preview/thumbs.rs` (`ensure_thumbs`, `ensure_waveform`, `ensure_preview_audio`) - every editor-media ffmpeg pass. Paired with `generate_once` (one pass at a time) so at most one below-normal ffmpeg runs, leaving the UI cores free.
