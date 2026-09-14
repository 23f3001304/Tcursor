# src-tauri/src/export/preview/segments_webcam.rs

Merge a take's webcam segments back into the single `webcam.webm` the editor's stage and the export's `WebcamPipe` already read (mid-take source switching, 2026-09-14). A camera switch writes `webcam_2.webm`, `webcam_3.webm`... beside the first file (`session::record::webcam_segments`); this runs once, from `preprocess::essential`, before the editor opens, so nothing downstream ever learns a second camera existed. A take with no switch has an empty `sync.webcam_segments` and pays nothing.

## Part

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Part { pub file: String, pub start_ms: u64, pub dur_ms: u64, pub w: u32, pub h: u32 }
```

One segment as the merge sees it: the file, where it starts on the recording clock, how long it actually ran (from `ffprobe`), and its pixel size.

The FIRST part's `start_ms` is `0`, by the same convention the rest of the codebase already aligns `webcam.webm` with: the export seeks the webcam by the take's `video_start`, i.e. it treats the webcam file's first frame as the take's start. `sync.json` has no `webcam_ms` to say otherwise, so the merge uses the one anchor the readers use. Every later part's `start_ms` is the switch instant `mark_webcam_segment` stamped.

## gaps

```rust
pub fn gaps(parts: &[Part]) -> Vec<u64>
```

The gap (ms) in front of each part.

### Returns

A vector the same length as `parts`. Element 0 is always `0`; element k is how long the camera was dark between part k-1 ending and part k starting - the device teardown and setup, typically 100 to 300 ms.

Saturating: a stamp that lands INSIDE the previous segment (a chunk still flushing when the switch was marked) closes the gap to zero rather than going negative, and the running end is `max(start, previous end) + dur` so the segment after that one is still placed after both instead of being re-overlapped.

### Behaviors (pinned by `segments_webcam_tests.rs`)

- `a_gap_is_the_dark_stretch_between_one_segment_ending_and_the_next_starting`.
- `a_stamp_inside_the_previous_segment_closes_the_gap_instead_of_going_negative`.

## merge_args

```rust
pub fn merge_args(parts: &[Part], out: &Path) -> Option<Vec<String>>
```

The whole ffmpeg argument list for the merge. Pure, so the filter graph is unit-tested without shelling out.

### Returns

`None` when there is nothing to merge (fewer than two parts) - the caller then leaves the take exactly as it is.

Otherwise one `ffmpeg` invocation: every part as an `-i` input in order, then a `filter_complex` that

- scales and pads each part into the FIRST part's size (`scale=W:H:force_original_aspect_ratio=decrease,pad=W:H:(ow-iw)/2:(oh-ih)/2`), because a second camera is routinely another resolution and `concat` demands one size;
- normalises every part to `MERGE_FPS` (30), `setsar=1` and `yuv420p`, which `concat` also demands;
- inserts a `color=c=black:size=WxH:rate=30:duration=<gap>` source before any part with a gap in front of it, so the merged file's timeline still matches the take's;
- `concat`s the lot in chronological order into `[out]`.

Output is VP9 (`libvpx-vp9`, `-b:v 0 -crf 32 -row-mt 1 -deadline good -cpu-used 4`) with `-an`: the webcam stream is video-only (the preview asks `getUserMedia` for video), and `good`/4 keeps a Stop the user is waiting on from turning into a long encode.

### Behaviors (pinned by `segments_webcam_tests.rs`)

- `one_segment_alone_has_nothing_to_merge`: one part, and zero parts, both give `None`.
- `two_segments_with_a_gap_and_a_size_mismatch_pad_into_the_first_segments_size`: both inputs in order, a black source at the first segment's size for the hole, `[v0][g1][v1]concat=n=3`, the 1280x720 camera fitted into 640x480 rather than stretched, VP9 out, the given output path last.
- `back_to_back_segments_need_no_black_at_all`: no `color` source and `concat=n=2` when the stamps line up.

## merge_webcam_segments

```rust
pub fn merge_webcam_segments(paths: &ProjectPaths, sync: &SyncLog) -> Result<(), String>
```

Replace `webcam.webm` with every segment merged in order.

### Inputs

- `paths` - the project, for `webcam()` and the folder the extra segments sit in.
- `sync` - the take's `sync.json`; only `webcam_segments` is read.

### Returns

`Ok(())` and does nothing at all when `sync.webcam_segments` is empty (every take without a camera switch) - and also when, after probing, fewer than two segment files actually survive on disk.

Otherwise: one ffmpeg pass into a `tmp_sibling` of `webcam.webm`, then the originals are moved into `<folder>/segments/`, then the temp file is renamed over `webcam.webm`.

- *Why a temp sibling and a rename:* no reader ever sees a half-written `webcam.webm`, the same rule `ensure_proxy`/`ensure_waveform` follow.
- *Why the originals are kept:* the merge is re-runnable by hand, and a bad merge never destroys a take's camera track. They are moved BEFORE the rename, so the first segment survives being overwritten by its own merged replacement.
- *Why a missing or unprobeable segment is skipped rather than fatal:* a take that lost one camera file should still get the rest of its camera, and the caller (`preprocess::essential`) treats a hard failure as "the first segment stands alone" anyway - the take is never lost.

### Behaviors (pinned by `segments_webcam_tests.rs`)

- `a_take_without_a_switch_is_untouched`: an empty segment list leaves the file byte-for-byte.
- `two_real_segments_merge_into_one_file_whose_duration_includes_the_gap` (guarded by `ffmpeg_present()`, skipped with a note when there is no ffmpeg on PATH): two generated 0.5s clips at different sizes with a 0.5s hole between them merge into one ~1.5s file at the first segment's size, with both originals under `segments/` and the extra one gone from the project root.

### Used by

- `export::preview::preprocess::essential` - called before the proxy, alongside `merge_mic_segments`, by the coordinator's wiring (plan Task E).
