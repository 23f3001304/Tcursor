# src-tauri/src/encode/vfr_segments.rs

Pause-aware VFR recording sink for the legacy (compatibility) capture path.

That path stamps the video by handing raw BGRA to ffmpeg's rawvideo demuxer under `-use_wallclock_as_timestamps 1`, which timestamps each frame with the instant **ffmpeg** reads it. There is therefore no timestamp to rebase the way the GPU path rebases `send_frame`: a paused span - during which nothing is written - is baked into `video.mp4` as a real PTS gap, and no ffmpeg option can take it back out, because a write can only ever happen at or after the moment the frame was captured (compressing the timeline in-stream would mean writing into the past). Meanwhile `sync.json`, the WAVs and every input stream DO drop the pause, so the export - which reads `video.mp4` 1:1 against `sync.json`'s clock - gets a pause-length frozen span, everything else running ahead of the picture from the resume on, and a truncated tail (finding C1).

So each unpaused span becomes its own MP4 part and `finish` concat-copies the parts into one file: the paused spans never exist in it at all. Each part carries an explicit `duration` directive computed from the recording clock, so the parts land **exactly** where `sync.json` says they should - see `concat_list` for why the container's own duration is not good enough. **Part 0 IS `out_path`**, so a recording that is never paused writes exactly one file and finishes with no remux and no extra cost - the un-paused behaviour is byte-for-byte what it was.

## Part

```rust
pub struct Part {
    pub path: PathBuf,
    pub first_ms: Option<u64>,
}
```

One recorded span.

- `path: PathBuf` - the part's file. `parts[0].path` is normally `out` itself.
- `first_ms: Option<u64>` - the RECORDING-clock timestamp of this part's first encoded frame (`RecordingSession::pump_once` rebases `Frame::ts` before pushing, so `push` can just read it). `None` until a frame arrives; such a part is dropped by `close_current` rather than reaching the concat list.

## part_path

```rust
pub fn part_path(out: &Path, n: usize) -> PathBuf
```

Path of segment `n` beside `out` (`video.mp4` -> `video.part1.mp4`). Segment 0 is normally `out` itself, so `n` is normally >= 1 - but a leading span that never received a frame is dropped, after which the numbering can restart at 0. Pure; unit-tested.

### Behaviors
- `part_paths_are_numbered_siblings_of_the_output`: `C:/rec/video.mp4` + 1 -> `C:/rec/video.part1.mp4`; + 12 -> `video.part12.mp4`.

## quote

```rust
fn quote(p: &Path) -> String
```

A path as one concat-demuxer single-quoted token. Backslashes become forward slashes **first**, so a Windows separator is never read as an escape; a literal `'` is then written as `'\''` - close the quote, escape the quote, reopen it.

*Why not the obvious `\'`:* inside a single-quoted token ffmpeg's `av_get_token` treats a backslash as an ORDINARY character, so `\'` ends the token early and silently truncates the path. Verified against the bundled ffmpeg with a real two-part join: a path under `O'Brien` written with `\'` resolved to `O\Brien` and the join failed outright (which would leave `video.mp4` holding only the pre-pause span while the salvaged `sync.json` described the whole take); the `'\''` form joined cleanly. The replacement order matters too - doing quotes first would then turn the inserted backslash into a slash.

## concat_list

```rust
pub fn concat_list(parts: &[Part]) -> String
```

The ffmpeg concat-demuxer list for `parts`, one `file '<path>'` line each, in order.

Every part but the last also carries a `duration` directive, which is what the demuxer offsets the FOLLOWING file by. Without it the offset comes from the part's own container duration, whose final-frame length is a VFR guess - measured at ~50 ms with this encoder on a real two-part wallclock-stamped join, and larger whenever the pre-pause span ended on a static screen, with the error accumulating across pauses against `full_dur_ms`. The directive is instead the recording clock's own span between consecutive parts' first frames (`next.first_ms - this.first_ms`), which is by definition where `sync.json` puts that part's first frame.

Measured on the real production command shape (two `-use_wallclock_as_timestamps` rawvideo parts around a 2 s pause): without the directive part 1 landed 50 ms early; with it, +0 ms. The directive is also verified not to truncate - it only moves the following file's offset - and the span is always at least the part's own content extent plus the boundary gap, so it can never place the next part on top of this one.

A part with no `first_ms` emits no directive (that boundary falls back to the container duration); in practice such a part never reaches here.

### Behaviors
- `concat_list_uses_forward_slashes_and_escapes_quotes_by_reopening`: `C:\O'Brien\video.mp4` -> `file 'C:/O'\''Brien/video.mp4'`, and asserts the un-parsable `\'` form is absent.
- `concat_list_is_one_line_per_part_in_order`: three parts produce three lines in the given order.
- `concat_list_carries_the_sync_clock_span_as_each_part_duration`: first frames at 25/1555/3555 ms produce `duration 1.530` and `duration 2.000`, and nothing after the last part.
- `duration_directives_render_millisecond_precision`: 7 ms -> `duration 0.007`; 61002 ms -> `duration 61.002`.
- `a_part_without_frames_emits_no_duration_directive`.

## VfrSegments

```rust
pub struct VfrSegments {
    sink: Option<FfmpegFrameSink>,
    out: PathBuf,
    parts: Vec<Part>,
    width: u32,
    height: u32,
}
```

- `sink: Option<FfmpegFrameSink>` - the part currently being written. `Option` because `FrameSink::finish` consumes a `Box<Self>`, so closing a part means taking it out. Kept in lockstep with `parts.last()`.
- `parts: Vec<Part>` - every part in order. `len() < 2` at `finish` is the "never paused" fast path.
- `width / height` - remembered so each new part spawns an identically-configured ffmpeg, which is what makes the parts concat-copyable.

## VfrSegments::new

```rust
pub fn new(out_path: &str, width: u32, height: u32) -> io::Result<Self>
```

Opens part 0 directly on `out_path`.

## VfrSegments::close_current

```rust
fn close_current(&mut self) -> io::Result<()>
```

Finalizes the part in progress (closing ffmpeg's stdin and waiting for it to write the `moov` atom), or does nothing if there is none.

If that part never received a frame, it is popped and its file deleted, and the finalize result is discarded: ffmpeg exits non-zero when its input carried no frames at all, and that is not a recording failure - it is a span the user paused straight back out of. It also must not reach the concat list, where it has no `first_ms` to build a `duration` from and would be a zero-frame MP4 for the demuxer to chew on.

## VfrSegments::join_parts

```rust
fn join_parts(&self) -> io::Result<()>
```

`-c copy` remux of the parts over `out`, via `ffmpeg -f concat -safe 0 -i <list> -c copy`. Written to a sibling (`video.joined.mp4`) and renamed over `out` only on success, so a failed join leaves `video.mp4` as the valid - if short - first part plus every other part still on disk. On failure the half-written `video.joined.mp4` is removed (it is not salvage; the parts are). The list file is removed either way; the extra parts are removed only after a successful rename.

## VfrSegments::push

```rust
fn push(&mut self, f: &Frame) -> io::Result<bool>
```

Writes to the current part and, on the first frame actually written, records that part's `first_ms` from `f.ts`. `f.ts` is the RECORDING clock, not the raw capture clock - `RecordingSession::pump_once` rebases it before pushing - which is what makes those timestamps the right basis for the concat offsets. Reports `Ok(false)` (the sink's "skipped, do not count or timestamp it" signal) if there is no open part.

## VfrSegments::split

```rust
fn split(&mut self) -> io::Result<()>
```

The pause -> resume boundary: finalize the current part and open the next one. Called by `RecordingSession::run`, once per resume.

## VfrSegments::finish

```rust
fn finish(mut self: Box<Self>) -> io::Result<()>
```

Finalizes the current part, then dispatches on how many parts actually hold frames: zero - nothing was ever encoded, so there is nothing to write; one - that span IS the whole recording, so it is left alone when it is already `video.mp4` (the never-paused case) and renamed into place when it is not (every span before it was empty); more - `join_parts`.
