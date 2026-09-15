# External review of TCursor: verified and expanded

2026-09-15. Branch `feat/layout-arrangements` at cb79d5d9, plus the uncommitted Batch A/B work in
`ports/` and `platform/`. Every factual claim below carries a `file:line` from this tree. Opinions
are labelled. Measured numbers are dated.

Three documents on this branch already settle parts of the review and are cited rather than redone:
`docs/codebase-cleanup-audit.md` (v2), `docs/cross-platform-architecture.md` (v2), and
`docs/readability-and-system-improvements.md` (being verified in parallel; it carries the detailed
verdicts on claims 2.D and 2.E).

Owner rulings taken as given today: macOS and Linux are funded with full parity; the cursor is
always a separately captured and redrawn layer; the hard 200-line cap is retired for per-kind
budgets; comments are being stripped in favour of the `docs/api/<path>.md` pages.

Headline: the review is **directionally right about the engineering and wrong about most of the
mechanisms**. It names real features but attributes them to the wrong implementations, and three of
its seven scores rest on claims this tree contradicts.

---

## Section 1 - "What makes it exceptional"

### 1.A Screen recording architecture - PARTLY TRUE (5 of 7 sub-claims hold)

| Sub-claim | Verdict | Evidence |
|---|---|---|
| Captures the display cursor-free | **VERIFIED** | `session/record/recorder.rs:141` passes `with_cursor: false` unconditionally; `switch_display.rs:41` does the same for a mid-take switch. Both capture paths honour it: `gpu_record.rs:75` and `capture/windows_capture.rs:90` select `CursorCaptureSettings::WithoutCursor`. The reason is documented at `recorder.rs:113-115` and `settings/cursor.rs:8-13`. |
| "at 60 FPS" | **PARTLY TRUE** | The rate is the primary display's refresh rate capped at 60, not a fixed 60: `recorder.rs:97` is `primary_refresh_hz().min(60)`, and `win/sys/display.rs:4-21` reads `EnumDisplaySettingsW`, falling back to 60. On a 144 Hz panel the capture is 60; on a 50 Hz one it is 50. It is also a *minimum update interval*, not a fixed cadence: `gpu_record.rs:76-77` sets `MinimumUpdateIntervalSettings::Custom(1_000_000/fps)`, so WGC delivers on content change and the take is VFR. |
| Logs global mouse coordinates | **VERIFIED** | `events/track/tracker.rs:52-106`, a `WH_MOUSE_LL` low-level hook plus its own message pump, with a 60 Hz poll feeding the same stamp site (`cursortracker.rs:88`, `tracker.rs:22-24`). |
| Extracts live OS cursor bitmaps via GDI | **VERIFIED** | `events/track/cursorcapture.rs:20-48`: `GetIconInfo` then `GetDIBits` against a borrowed screen DC. Documented limit at `:17-19`: animated `.ani` cursors are sampled once and render as a still. |
| Separate "vector/bitmap timeline" | **PARTLY TRUE** | Bitmap, never vector. The captured layer is raw pixels (`events/track/cursorlayer.rs:97`) and every pack is PNG plus a `hotspots.json` (`export/cursor/pack.rs:1-6`; `src-tauri/assets/cursorpacks/aero-glass/*.png`, 12 bundled packs). There is no SVG or vector path anywhere in the cursor code. |
| "resize the cursor up to 300%" | **VERIFIED** | The range is 0.4x to 3.0x, clamped identically in Rust (`export/cursor/cursordraw.rs:180`, `cursormorph.rs:78`) and in the UI (`src/editor/panels/CursorPanel.tsx:105`, `min={0.4} max={3.0}`). 300% is the true ceiling; the review omits that 40% is the floor. |
| Motion blur, custom packs, click animations, no re-encode of the video | **VERIFIED** | `CursorSettings` at `settings/cursor.rs:30-46` carries `motion_blur`, `pack`, `click_bounce`, `bounce_intensity`, plus `smoothness`, `path_idealize`, `tilt` and `back` the review did not mention. All are editor-side; the recorded `video.mp4` is untouched (`gpu_record.rs:2-4`). |

### 1.B Camera choreography and motion physics - LARGELY WRONG

| Sub-claim | Verdict | Evidence |
|---|---|---|
| "damped spring physics" | **WRONG as stated** | There is no spring. The follow is an exponential low-pass with a per-millisecond time constant (`export/camera/follow.rs:8-20`). The optional post-pass is an *analytic critically damped second-order filter*, and its own doc comment at `export/camera/smoothing.rs:15-18` explicitly contrasts it with "the explicit Euler spring, which rings then diverges once `w * dt` passes ~0.5". Calling it a spring inverts the design decision. |
| "cubic Bezier curves" | **VERIFIED** | `export/keys.rs:13` `KeyMode::Bezier`, `:107-127` a Bernstein-form cubic with a bisection solve for the monotone x. `:130-132` records that springs are deliberately *not* offered because they overshoot. |
| "anti-jank jitter filters" | **PARTLY TRUE, and off by default** | The filter exists (`smoothing.rs`) but `ZoomConfig::smoothing_ms` defaults to `0` (`export/types.rs:31`), and `0` is a documented identity (`smoothing.rs:41`). The six `jank_*` files in `export/camera/` are an *investigation harness*, not a shipped filter: `jank_scene.rs` is a synthetic 12 s scenario, `jank_metrics.rs` computes jerk series, and `jank_probe_tests.rs`/`jank_phase_tests.rs`/`jank_input_tests.rs`/`jank_filter_tests.rs` are tests. |
| "automatically detects hesitation / jerky mouse movement" | **WRONG as a camera claim, PARTLY TRUE as a cursor one** | No hesitation detector feeds the camera; a grep for `hesitat`/`jerky` over `src-tauri/src` returns nothing outside `ai/`. What exists is the offline cursor path model: `export/cursor/path.rs:1-9` splits the recorded route into RESTS (the hand held still within `rest_px` for `REST_MS = 120`, `path.rs:10-11`) and MOVES, and idealizes the moves only with both ends pinned. That is rest detection, not hesitation smoothing, it acts on the drawn cursor rather than the camera, and `path_idealize` defaults to `0.0` (`settings/cursor.rs:52`) so it is off unless asked for. |
| "solves the multi-monitor coordinate shift" | **VERIFIED** | `export/coordmap.rs:6-8` subtracts the monitor origin; `session/record/target_bounds.rs:28-40` records the exact regression (indexing `EnumDisplayMonitors` used a different order from `Monitor::from_index`, so `display:N` took another monitor's origin and shifted every cursor point) and fixes it by correlating GDI device names. |
| "and DPI scaling issues" | **WRONG / no evidence** | A case-insensitive grep for `dpi`, `per-monitor` and `awareness` over `src-tauri/` returns one unrelated hit (`ai/plan/json.rs:58`). There is no DPI-awareness manifest, no `GetDpiForMonitor`, no scale-factor handling. Mixed-DPI may work because WGC and `WH_MOUSE_LL` both report physical pixels, but nothing in the tree addresses it and nothing tests it. |

### 1.C Zero-copy GPU pipeline - VERIFIED, with one omission

- **Zero-copy WGC to Media Foundation: VERIFIED.** `gpu_record.rs:1-4` states it ("feed each WGC frame's D3D11 surface straight into the windows-capture Media Foundation `VideoEncoder` (no GPU->CPU readback)") and `gpu_frames.rs:173-180` proves it: the surface and texture are taken by `as_raw_surface()`/`as_raw_texture()` and rebuilt into a `Frame` handed to `enc.send_frame`. The readback buffer `Frame::new` demands is explicitly never read (`gpu_frames.rs:66-68`).
- **The omission.** There is a second, non-zero-copy path. `capture/windows_capture.rs:66-78` calls `buf.as_raw_buffer().to_vec()`, a full CPU copy per frame, feeding the ffmpeg pipe. It is the fallback for the `game_mode` compatibility toggle and for GPU-encoder init failure (`recorder.rs:134-136`). Both pipelines are live and maintained; the review describes only one.
- **`frame_scaler.rs` and mid-recording resize: VERIFIED.** `session/record/frame_scaler.rs:1-4` is exactly the claimed design: one persistent D3D11 canvas at the encoder's size, with `ID3D11VideoProcessor` fitting every differently-sized capture frame into it, so a mid-record resize neither ends the take nor freezes the picture. "OBS-style" is the file's own word (`:1`).
- **WGSL on wgpu: VERIFIED but over-scoped.** `wgpu = "22"` (`src-tauri/Cargo.toml:32`); four shaders exist (`export/gpu/shader.wgsl`, `export/fx/fx.wgsl`, `fx_clicks.wgsl`, `fx_lens.wgsl`). Rounded corners are WGSL (`shader.wgsl:63-73`, a signed-distance rounded rect). **Background blur is not**: it is a CPU separable box blur at `export/scene/background.rs:79-110`. **Drop shadows are not in the compositor either**: the only shadows are the cursor lens's (`export/fx/fx_lens.wgsl:20`, `fx_lensdraw.rs:16-31`) and a 1 px text shadow (`export/fx/captiondraw.rs:90`). `shader.wgsl` has no shadow term at all. Timeline composition is correct.

### 1.D AI director - PARTLY TRUE (one claim wrong)

- **Structured event transcripts: VERIFIED.** `ai/plan/transcript.rs:28-100` emits clicks with a nine-region label (`:6-14`), typing bursts merged under a 1 s gap (`:55-66`), text-field spans inferred from `CursorType::IBeam` runs (`:69-84`), and idle gaps over 2 s (`:86-90`), capped at 120 moments (`:93-95`).
- **"window switches": WRONG.** No window-switch or app-switch detection exists. The transcript's only `window` token is `times.windows(2)`, a slice iterator (`transcript.rs:87`). The prompt's own input grammar (`ai/llm/prompt.rs:22-28`) lists six line kinds and none is a window switch.
- **Sampled keyframes: VERIFIED.** `ai/frames/sample.rs:5-49` samples at clicks, layout changes, and an 8 s idle grid, with real events winning over grid slots; `ai/frames/extract.rs:13-27` pulls each as a 512 px-long-edge mjpeg via ffmpeg from the **proxy**, not `video.mp4`, so frame times are already on the output clock (`extract.rs:29-31`).
- **Local Ollama vision models: VERIFIED.** `ai/llm/ollama.rs:57-63` puts base64 images on the user message (never on the request, noted at `:7-9`); `ai/commands.rs:15-19` lists installed models and probes each for vision.
- **Suggest zoom targets and cut dead air: VERIFIED and understated.** The wire schema (`ai/llm/prompt.rs:2-10`) has six edit kinds: `zoom`, `layout`, `spotlight`, `cut`, `speed`, `trim`. The review names two.

---

## Section 2 - "What holds it back"

### 2.A Deep OS lock-in - TRUE in substance, WRONG in scale

`30+ files` is an overcount. Files in `src-tauri/src` that reference `windows::`, `windows_capture::`, `wgc_windows::` or carry a `cfg(windows)`: **20**, of 364 `.rs` files (grep, 2026-09-15). That matches `docs/cross-platform-architecture.md` section 3, which inventories 20 distinct API surfaces across 19 files and is the authoritative list.

Two corrections that matter more than the count, both from that document and re-confirmed here: `commands.rs:13` and `target_bounds.rs:60` call `windows_capture::monitor::Monitor` with **no `cfg` guard**, so those files do not compile off Windows regardless of the `#[cfg(windows)]` blocks near them; and 35 of the 52 `crate::win::` call sites were `win::sys::proc`, which is not platform code at all. That file has already moved (`process/proc.rs`, Batch B, logged at `docs/cross-platform-architecture.md:483`).

The claim that the review does *not* make, and should: `export/` is already portable end to end, `cpal` already covers three platforms for the microphone, and `wgpu` already covers three for compositing. The lock-in is confined to capture, input hooks, and four small system calls.

### 2.B "200-line code compression" syndrome - PARTLY TRUE, largely refuted

| Sub-claim | Verdict | Evidence |
|---|---|---|
| Files capped at 200 lines | **VERIFIED as a fact, now obsolete as a criticism** | Zero `.rs` files in `src-tauri/src` exceed 200 lines; two `.ts` test files sit at exactly 201. The owner retired the hard cap for per-kind budgets today, so the constraint the review attacks no longer exists in that form. |
| Multiple statements per line | **PARTLY TRUE, modest** | 176 non-test lines carry two or more semicolons, 32 carry three or more (grep excluding `for` headers and comments). Out of roughly 20k non-test Rust lines that is under 1%. |
| Cryptic variable names | **PARTLY TRUE, and the four examples are weak** | `cands` is a documented parameter of a 2-line function (`process/proc.rs:71-72`). `ii` is an `ICONINFO`, the Win32 convention, in a 15-line unsafe block (`events/track/cursorcapture.rs:22-33`). `enc` outside tests is an `EncoderSpec` field (`gpu_record.rs:130`) and a loop variable in `encode/ffmpeg_args.rs:75`. `cs`/`ce` are clip-start and clip-end inside one 6-line loop (`export/remap.rs:59-62`) whose surrounding type names them in full (`Segment { clip_start, clip_end }`, `remap.rs:13`). The instinct is defensible as a style opinion; these four citations do not carry it. |
| Anonymous tuples instead of structs | **PARTLY TRUE** | Real instances exist, e.g. `CursorSamples = (Vec<(u32, CursorType)>, CursorLayerBuilder)` (`events/track/cursortracker.rs`, used by `ports/input.rs:40`). Not quantified by the review and not quantified here. |
| "110 fragmented micro-test files" | **VERIFIED as a count, WRONG as a cost claim** | Now 118 `*_tests.rs` (grep, 2026-09-15). `docs/codebase-cleanup-audit.md:11-13, 184-192` settles the cost: all are `#[path]`-included modules of one crate, `cargo test --lib` builds one binary, and merging them would save approximately nothing. The 38.0 s test-code rebuild cost tracks lines, not files. |
| "compensated by paragraph-long war-story comments" | **VERIFIED as description, and it is the deliberate policy being reversed** | e.g. `session/record/pause_totals.rs:1-21`, 21 lines of comment before the first `use`, recording the exact bug (WGC delivers frames only on content change, so a pause over a static desktop measured 0 ms). The owner has ruled today that these move to `docs/api/<path>.md`. Opinion: the *content* of those comments is the single most valuable non-code asset in the repo, and the migration should preserve it verbatim rather than summarise it. |

### 2.C Lack of clean abstractions - LARGELY REFUTED, and already being fixed

All three specific charges are answered in `docs/cross-platform-architecture.md` section 2, and this tree now contradicts the framing:

- **`enum VideoSink`**: `session/record/video_sink.rs:25`. The enum does not encode platforms; `Dead` holds the frame timestamps of a capture a failed display switch killed so Stop can still write a truthful `sync.json` (`video_sink.rs:34-38`). See `docs/cross-platform-architecture.md:42-51`.
- **`start_recording` as a god function**: `session/record/recorder.rs:74-174`, 101 lines. Both things it does beyond composing are bugs it fixed: inputs start before the encoder on purpose (`recorder.rs:134-136`), and the error arm at `:143-158` joins the mic and system-audio threads it already spawned, without which they leak with a device open and a WAV unfinalized. See `docs/cross-platform-architecture.md:64-74`.
- **`static SINK`**: `events/track/tracker.rs:13`. It cannot be removed: `SetWindowsHookExW` takes a bare `unsafe extern "system" fn` with no user pointer. Three alternatives are weighed and rejected at `docs/cross-platform-architecture.md:89-103`. **`FFMPEG_DIR`** is a `OnceLock<PathBuf>` set once at startup, not mutable state. `GEN` (`process/proc.rs:19`) is a deliberate fix, not a smell.
- **"prevent clean unit testing"**: partly true, and the remedy is in flight. `ports/` and `platform/` now exist in this tree (`ports/capture.rs`, `ports/input.rs`, `ports/system.rs`, `platform/windows/capture.rs`, `platform/windows/system.rs`), which is Batch A of the plan; `platform/mock/` plus the first headless recorder test is Batch D.

### 2.D High scrub latency - TRUE in mechanism, WRONG in numbers and in scope

Quick independent check; the detailed verdict lives in `docs/readability-and-system-improvements.md`.

- **The ffmpeg subprocess is real.** `export/preview/encode.rs:11-20`: `jpeg_encode` writes the BGRA buffer to a temp file through `StagedInput` (`export/pipeline/ffio.rs:57-67`, a plain `std::fs::write`) and spawns `ffmpeg` to produce the mjpeg. The file's own comment gives the reason: a PNG deflate in a debug build costs more than the render, and the JPEG is a tenth of the bytes over IPC.
- **"8-33 MB" is wrong.** The preview canvas is capped at a 1280 px long edge (`export/preview/mod.rs:24`), so the staged buffer is about 3.7 MB at 1280x720 and 4.1 MB at 1280x800, not a full-resolution frame.
- **"every timeline scrub" is wrong.** The live scrub and playback path is a client-side canvas composite from the proxy (`src/editor/hooks/useCompositeLoop.ts:24-27`), not an IPC round trip. The Rust exact frame is requested when playback pauses or a scrub settles (`export/preview/encode.rs:7-9`), and the FX overlay is already rate-limited and half-resolution after exactly this stutter was diagnosed (`useCompositeLoop.ts:17-22`).
- **What the review missed here**: the renderer is warm-cached per recording and keyed by `(folder, edit.json mtime, aspect)` (`export/preview/mod.rs:2-6`), so only seek-decode plus composite plus encode rerun.

### 2.E Security gaps - VERIFIED

`src-tauri/tauri.conf.json:30` is `"csp": null` and `:33` ends the asset-protocol scope with `"**/*"`, which makes the five preceding `$HOME`/`$VIDEO`/`$DESKTOP`/`$DOCUMENT`/`$DOWNLOAD` entries redundant and grants the webview read access to any path via `convertFileSrc`. Mitigating context the review omits: `src-tauri/capabilities/default.json` grants no `fs:` permissions at all, so there is no JS filesystem API, and every file read goes through one of the 50 registered commands. The exposure is a read primitive reachable only if something can inject script into the webview, which `"csp": null` makes cheaper than it should be. Detailed verdict and remedy: `docs/readability-and-system-improvements.md`.

---

## Section 3 - Market claims

**All of these are opinions or external facts; none is checkable from this tree. Labelled as such.**

- "OBS is cross-platform and powerful": opinion, and true to my knowledge.
- "OBS lacks automated zoom follow / cursor restyling / quick editing": **true to my knowledge** for stock OBS; third-party plugins and Lua scripts add crude zoom-follow. UNVERIFIABLE from the tree.
- "Screen Studio is macOS only": **true to my knowledge** as of my May 2026 cutoff. UNVERIFIABLE from the tree, and worth re-checking before it is used in positioning.
- "Camtasia, CleanShot X placed as mid / single-OS": opinion. CleanShot X is macOS-only to my knowledge; Camtasia ships Windows and macOS, so "single-OS" is wrong for it.
- "TCursor has the potential to become the de facto cross-platform standard": opinion, and unfalsifiable.

---

## Section 4 - The scorecard

`"220+ test files"` is **VERIFIED as a count and understated**: 118 `*_tests.rs` + 121 `*.test.ts(x)` + 6 integration files in `src-tauri/tests/` = **245** (grep, 2026-09-15). The characterisation attached to it is what fails. "Many test compiler primitives": `docs/codebase-cleanup-audit.md:137-153` read every cited test in full and reversed 7 of 11 verdicts. "Depend on local developer paths": `docs/codebase-cleanup-audit.md:164` shows `manual_export.rs:21-24` prints "skipping" and returns; all five `manual_*.rs` are `#[ignore]`d and skip rather than fail.

### Corrected scorecard

| Metric | Theirs | Mine | One-line justification |
|---|---|---|---|
| Product vision and market fit | 9.5 | **8.5** | The positioning is real, but the score rests entirely on the unverifiable market claims in section 3, and the differentiator (auto-zoom, cursor restyling) is a feature set, not a moat. Opinion. |
| Domain and graphics engineering | 9.0 | **9.0** | Confirmed at a mechanism level the review did not reach: zero-copy WGC to MF (`gpu_frames.rs:173-180`), a D3D11 video-processor canvas that survives mid-take resizes (`frame_scaler.rs:1-4`), a per-millisecond camera time constant (`follow.rs:8-20`), and an analytic critically-damped filter chosen over an Euler spring for stability (`smoothing.rs:15-18`). |
| Feature completeness | 8.5 | **9.0** | The review misses time remap (`export/remap.rs`), the exact-span pause ledger (`pause_totals.rs`), on-device ASR captions (`asr/whisper.rs`, whisper-rs enabled 2026-09-15 at `Cargo.toml:58`), keyframed camera moves, layout arrangements, mid-take source switching, and 12 bundled cursor packs. |
| Code architecture and OOP | 5.5 | **7.0** | Every specific charge is answered with a recorded reason (`docs/cross-platform-architecture.md` section 2), and `ports/` + `platform/` already exist in this tree. Marked down from higher only because the composition root is not wired yet (Batch D) and there is still no headless recorder test. |
| Portability | 3.0 | **4.0** | 20 of 364 Rust files touch Windows APIs, not "100% locked"; `export/`, `encode/`, audio capture and the whole frontend are already portable. Two unguarded `Monitor` calls (`commands.rs:13`, `target_bounds.rs:60`) are what actually block a cross-compile. Now that macOS and Linux are funded, this is the score with the most headroom. |
| Readability and formatting | 5.5 | **7.5** | Zero files over 200 lines, zero `TODO`/`FIXME`/`HACK` in either tree, 478 docs/api pages with a validator that fails on drift (`docs/codebase-cleanup-audit.md:300-309`). Under 1% of non-test lines carry multiple statements. The genuine cost is that the reasoning lives in comments rather than in the docs pages, which is exactly what the owner is changing. |
| Test suite quality | 5.0 | **8.0** | 245 test files, 993 Rust tests and 998 vitest tests passing on this branch (`docs/codebase-cleanup-audit.md:410, 412`). Real oracles exist that the review never looked for: a CPU compositor used as the GPU shader's reference (`gpu_compositor_tests.rs:57, 91`), a 64-bit trajectory fingerprint (`export/camera/jank_filter_tests.rs:41`, `0x58da_34b7_41e9_0fd3`), and an nv12-against-ffmpeg pixel check (`export/color_tests.rs:32`). Marked down from higher because nothing on the record path can run headlessly. |

**Overall: 7.5 stands, for different reasons.** Their 7.5 averages a harsh architecture/portability/test read against a generous market one. Mine is a strong engineering and test read against a portability gap that is now funded and a product that is one milestone from complete.

---

## Section 5 - "A diamond in the rough" - AGREED with one correction

"Zero-latency GPU screen capture, smooth spring physics, timeline synchronization, WebGPU compositing, custom cursor extraction - already solved and working." Four of five hold. "Spring physics" is the correction from 1.B: the shipped motion is an exponential follow plus an optional, default-off analytic critically-damped filter, and springs were explicitly declined (`export/keys.rs:130-132`).

---

## What the review missed

### Strengths it does not mention

1. **Export is the reference, enforced structurally.** Preview and export pick the same compositor through the same function (`export/gpu/compositor.rs:72`, one call site at `export/render/mod.rs:96`), and the preview renderer is the export renderer at a smaller resolution (`export/preview/mod.rs:29-32`). This is the most important architectural decision in the codebase and the review does not see it.
2. **The time remap model.** `export/remap.rs:1-6`: one clip-to-output clock map where trim, cuts and speed spans become kept segments with a factor and an output start, mirrored expression-for-expression by `src/lib/remap.ts:3-7` in f64 with no `fround`, with a parity table pinned against a Rust fixture. Speed factors are clamped to 0.25..8.0 (`remap.rs:9-10`).
3. **The exact-span pause ledger.** `session/record/pause_totals.rs:1-21`. The pause instant is stamped by the same code path that flips the `paused` flag, so `sync.json`, the MP4's PTS, both WAVs and every event stream are compressed by the identical number. The 21-line comment records the bug it replaced: inferring the span from gaps between paused frame arrivals made a take's reported duration a function of what the screen happened to be doing. `paused_ms` and `pause_started` are one mutex-guarded unit specifically to make a torn read structurally impossible.
4. **On-device ASR.** `asr/` is 16 files; `whisper-rs 0.16` was enabled in `Cargo.toml:58` on 2026-09-15 with the CPU-only feature set on purpose (`Cargo.toml:39-41`), and `asr/whisper.rs:1-4` keeps it behind one seam so the rest of the milestone is testable without a model on disk. Thread count is capped at 8 so a background transcription cannot take the machine (`whisper.rs:13-18`).
5. **The salvage path.** `VideoSink::Dead` and the no-`Result` stop contract (`session/record/video_sink.rs:34-45`) exist because a finalize failure used to propagate before `sync.json`, the manifest and the recents entry were written, leaving a folder the app could not open.
6. **A CPU compositor as the shader's reference implementation**, not dead weight: `rrect_sd_px` is documented as the identical formula to `shader.wgsl`'s `rrect_sd`, and the GPU tests composite the same scene both ways and compare (`docs/codebase-cleanup-audit.md:96-116`).

### Weaknesses it does not mention (new; the three docs hold the known ones)

1. **A second compositor lives in TypeScript, and parity is maintained by hand.** `src/editor/stage/` holds 42 non-test modules including `previewCanvas.ts`, `previewDraw.ts`, `camera.ts`, `cursorTilt.ts`, `cursorGlass.ts`, `ripplePreview.ts`, `spotlightPreview.ts` and `captionPreview.ts`, all drawing the live stage in the browser. `useCompositeLoop.ts:78, 103, 107, 118` each note where the TS mirrors a specific Rust function (`step_camera`, `Cursor`'s filter, the panel mapping). This is the real cost of "export is the reference": the reference is Rust, but what the user watches while editing is a hand-kept TS copy of it, and 126 frontend files mention Rust or a mirror. Only `remap.ts` has a pinned parity fixture; the rest are kept in step by comment. **Opinion: this is the single largest correctness risk in the product**, and it is invisible to a reviewer who only reads Rust.
2. **The cross-platform cursor layer may not survive the port, and the editor promises it today.** `docs/cross-platform-architecture.md:590-594` flags that reading another application's cursor bitmap may be impossible on Wayland and unclear on macOS. Because the capture is *unconditionally* cursor-free (`recorder.rs:141`, no toggle), a platform with no cursor-shape port produces a take with no cursor at all rather than a degraded one. The Windows-only design has no fallback in it.
3. **`jpeg_encode` stages to disk on a user-visible path.** `export/pipeline/ffio.rs:65` is a `std::fs::write` to `%TEMP%` per exact-frame request, and the process-unique naming means a heavy editing session leaves a trail of files the `Drop` at `:72` only removes on a clean exit. A pipe to ffmpeg's stdin would remove both the write and the cleanup obligation.
4. **The AI transcript's 120-moment cap is silent to the model in a misleading way.** `ai/plan/transcript.rs:93-97` truncates by time order after sorting, so a long recording hands the director the first 120 moments plus a "(capped)" line, and it will never propose an edit in the back half of a busy clip. Sampling across the clip, as `ai/frames/sample.rs:43-49` already does for frames, would be consistent with the frame path.
5. **`primary_refresh_hz` is the capture rate even when the target is on another monitor.** `recorder.rs:97` and `switch_display.rs:40` both read the *primary* display's refresh rate, but the capture target can be any display (`target_bounds.rs:28-40`) and switches mid-take. On a mixed 144 Hz / 60 Hz desk, recording the secondary picks the wrong minimum update interval. Low impact because both are capped at 60, real if the secondary is a 30 Hz or 50 Hz panel.

---

## What would move each low score up

Cross-referenced so nothing is planned twice.

**Portability, 4.0 -> 7.0. Already planned; do not re-plan.**
`docs/cross-platform-architecture.md` section 6, Batches A to D. Batch B has landed (`:483`); `ports/` and `platform/` exist in this tree, so Batch A is in flight. The two highest-leverage items inside it, both cheap: fix the unguarded `Monitor` calls at `commands.rs:13` and `target_bounds.rs:60`, and add the cross-compile `cargo check --target aarch64-apple-darwin` gate (`:458-461`) that keeps Windows calls from leaking back. Now that macOS and Linux are funded, decision 1 (`:564-569`) is answered and Phases 2 and 3 are live, which makes the Wayland pointer question (`:542-550`) and the cursor-layer question (`:590-594`) the two spikes to run first, before the port design hardens.

**Architecture, 7.0 -> 8.5. Already planned.**
Batch D (`docs/cross-platform-architecture.md:496-506`): wire `Arc<Platform>` into `lib.rs::setup`, move `Running` to boxed ports, add `platform/mock/` and the first headless start/pause/resume/stop test. That single test is what turns the test-suite score and the architecture score at once.

**Test suite, 8.0 -> 9.0. Mostly planned; one gap.**
The cleanup audit's batches 1 to 4 are executed (`docs/codebase-cleanup-audit.md:410-414`). The remaining gap is not in either document: **the TS/Rust mirror modules have no parity fixtures except `remap.ts`**. The pattern to copy already exists - `remap.test.ts` pins a table computed from a Rust fixture. Applying it to `camera.ts` (against `step_camera`) and `cursorTilt.ts` (against `export/cursor/tilt.rs`) would close weakness 1 above at test cost only, with no refactor. Propose this as a new batch, sized at two agents on disjoint files.

**Readability, 7.5 -> 9.0. Being decided now.**
The comment-to-`docs/api` migration the owner ruled on today is the whole move. One constraint worth writing into that plan: the validator already fails on drift and orphans (`docs/cross-platform-architecture.md:167-170`), so the migration is enforceable, but the bug retrospectives in comments like `pause_totals.rs:1-21` and `target_bounds.rs:28-40` are the reason those files are correct. Moving them verbatim is the requirement; compressing them is a regression. Four sources had no docs page and Batch 4.2 added them (`docs/codebase-cleanup-audit.md:410`).

**Security, not scored by the review but load-bearing.**
Set a real `csp` and drop the `"**/*"` entry from the asset-protocol scope (`tauri.conf.json:30, 33`), keeping the five named roots. The detailed remedy belongs to `docs/readability-and-system-improvements.md`; it is listed here only so the two documents do not both claim it.

**Not worth doing, with reasons.**
Merging the 118 Rust test files (measures as zero saving, `docs/codebase-cleanup-audit.md:184-192`). Deleting the CPU compositor (it is the shader's oracle, `:96-116`). Removing the `TCURSOR_REC` tests (`:175-181`). Replacing the exponential follow with a spring (declined on record, `export/keys.rs:130-132`). Chasing the "god function" charge (`docs/cross-platform-architecture.md:64-74`).
