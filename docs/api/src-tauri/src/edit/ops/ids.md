# src-tauri/src/edit/ops/ids.rs

The next free id for each region list, moved out of `edit/ops/api.rs` verbatim for headroom (2026-09-15, M5 plan ADDED-7). Each is the highest numeric suffix on the list's own prefix plus one, else the list's length, so an id is never reused after a delete-then-add. `pub(crate)`: only the ops call them.

## next_zoom_id

```rust
pub(crate) fn next_zoom_id(doc: &EditDoc) -> String
```

`"z<n>"` for `doc.zooms`.

## next_layout_id

```rust
pub(crate) fn next_layout_id(doc: &EditDoc) -> String
```

`"l<n>"` for `doc.layout`.

## next_cam_id

```rust
pub(crate) fn next_cam_id(doc: &EditDoc) -> String
```

`"k<n>"` for `doc.camera_moves`.

## next_caption_id

```rust
pub(crate) fn next_caption_id(doc: &EditDoc) -> String
```

`"c<n>"` for `doc.captions`. The same prefix `asr::group::group_words` stamps on a freshly transcribed track, deliberately: a Split made after a transcription then keeps counting from where the track left off (`c12` after `c0..c11`) instead of minting a `c0` that already exists. The highest-suffix-plus-one rule is what makes that safe - it reads the ids that are actually there, not a counter.

### Used by

- `src-tauri/src/edit/ops/api.rs` - the `AddLayoutSeg` / `AddCameraMove` arms.
- `src-tauri/src/edit/ops/motion.rs` - `add_zoom` (the shared `AddZoom` / `AddZoomFull` body).
- `src-tauri/src/edit/ops/captions.rs` - `apply_caption`'s `SplitCaption` arm.
