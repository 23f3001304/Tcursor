# src-tauri/src/events/track/cursorlayer.rs

The recording's OS-cursor layer: the real cursor bitmaps, and a timeline of which one was on screen. Written to `cursor/layer.json` plus one `cursor/<id>.png` per shape (see `session/paths.rs`).

**Why a layer at all.** The screen is now always captured WITHOUT the OS cursor, for every style - `recorder.rs` passes `with_cursor: false` unconditionally. That is what makes the cursor style a real editor decision instead of a record-time one: "System" composites this layer at the raw recorded path, "Enhanced" draws the synthetic sprite stack, "Hidden" draws nothing, and switching between them in the editor changes nothing about the pixels underneath. Before this, a take recorded in Enhanced or Hidden had no OS cursor anywhere, so switching it to System could only ever produce a generic arrow.

**Legacy.** The file's mere presence is also the marker that separates a post-layer recording from a pre-layer one, whose System take has the cursor baked into the video and must NOT get a second one drawn on top. See `settings::store::os_cursor_in_video`.

## MAX_CURSORS

```rust
pub const MAX_CURSORS: usize = 64
```

Upper bound on distinct cursor bitmaps kept for one recording. A take that cycles through more (a custom-cursor game, a theme switcher) keeps the first 64 and reuses the last known shape afterwards. *Why a cap at all:* the layer is a fidelity aid, not a reason to let a project folder grow one PNG per exotic cursor a long session happens to touch.

## CursorEntry

```rust
pub struct CursorEntry { pub id: u32, pub w: u32, pub h: u32, pub hx: u32, pub hy: u32, pub file: String }
```

One captured cursor in `layer.json`. `id` is the entry's index (dense, stable within a take, and the value the track refers to); `w`/`h`/`hx`/`hy` come straight from the `CapturedCursor`; `file` is the PNG's name relative to `ProjectPaths::cursor_dir` (`"<id>.png"`). The size and hotspot live in the JSON rather than being re-derived from the PNG so a reader can size and place a cursor without decoding it.

## CursorLayer

```rust
pub struct CursorLayer { pub cursors: Vec<CursorEntry>, pub track: Vec<(u32, u32)> }
```

The deserialized layer: the entries, plus `(t_ms, id)` samples saying which cursor was showing from when. Times are paused-aware recording-clock milliseconds - the same `PauseTotals::stamp` values `CursorTrack` uses - so both tracks index the same clock.

One sample is appended per CHANGE, not per poll.

### Used by

- `src-tauri/src/export/cursor/captured.rs` - decodes the entries into sprites and looks up `id_at` per frame
- `src-tauri/src/export/cursor/cursorpreview.rs` - `cursor_layer` serves it to the editor preview
- `src-tauri/src/settings/store.rs` - `os_cursor_in_video` uses `exists` as the pre/post-layer marker

## CursorLayer::exists

```rust
pub fn exists(paths: &ProjectPaths) -> bool
```

Whether this recording captured a cursor layer at all - i.e. whether its video was captured clean.

File existence ONLY: no parse, no entry count. An EMPTY layer still means a clean video (every `GetIconInfo` could have failed while the capture was still cursor-free), and treating that as "no layer" would tell the editor the cursor is baked into pixels that do not contain it, leaving System permanently blank.

## CursorLayer::load

```rust
pub fn load(paths: &ProjectPaths) -> Option<Self>
```

The layer, or `None` for a recording that has none. A missing file, an unreadable one and corrupt JSON are all the same answer - never an `Err`, never a panic: a missing cursor degrades to the synthetic fallback, it does not fail opening a project.

## CursorLayer::id_at

```rust
pub fn id_at(&self, t_ms: u32) -> Option<u32>
```

The cursor id showing at `t_ms` - the last sample with `t <= t_ms`, found by `partition_point`. Mirrors `CursorTrack::type_at`, except that it returns `None` rather than a default before the first sample and on an empty track: there is no universal fallback bitmap the way `Arrow` is a universal fallback shape, so "nothing captured yet" has to mean "draw nothing".

The TypeScript mirror is `idAt` in `src/editor/stage/cursorPreview.ts`.

## CursorLayerBuilder

```rust
pub struct CursorLayerBuilder { entries: Vec<(CursorEntry, Vec<u8>)>, track: Vec<(u32, u32)> }
```

Accumulates the layer DURING a recording - the entries still paired with their pixels, since the PNGs are only written at stop. Lives on the cursor-polling thread and is handed back by `CursorTypeTracker::stop` alongside the shape samples.

Kept separate from `CursorLayer` so the serialized shape never has to carry pixel buffers, and so the "add / mark / save" recording API is not exposed to readers.

## CursorLayerBuilder::is_full

```rust
pub fn is_full(&self) -> bool
```

True once `MAX_CURSORS` bitmaps are held. The tracker checks this BEFORE calling `cursorcapture::capture`, so past the cap no further GDI work is done at all.

## CursorLayerBuilder::add

```rust
pub fn add(&mut self, c: CapturedCursor) -> u32
```

Store a captured bitmap and return its id (its index). Called once per distinct `HCURSOR` handle a take sees.

## CursorLayerBuilder::mark

```rust
pub fn mark(&mut self, t_ms: u32, id: u32)
```

Record that cursor `id` is the one showing from `t_ms` on. The tracker calls this only when the shown id actually changes, so the track stays one entry per change.

## CursorLayerBuilder::layer

```rust
pub fn layer(&self) -> CursorLayer
```

The serializable layer without the pixels - what `save` writes, and what tests assert on without touching disk.

## CursorLayerBuilder::save

```rust
pub fn save(&self, paths: &ProjectPaths) -> std::io::Result<()>
```

Create `cursor/`, write one RGBA8 PNG per entry (via the same `png` crate the preview encodes frames with - no new dependency), then write `layer.json`.

Written even when NOTHING was captured, on purpose: the file's presence is the record that this take's video is cursor-free, and skipping it would make the project look pre-layer forever.

### Used by

- `src-tauri/src/session/record/recorder_threads.rs` - `save_inputs` calls it at stop, best-effort (a failure is logged, never fails the stop; the project then simply falls back to the plain arrow for System, exactly like a pre-layer recording)
