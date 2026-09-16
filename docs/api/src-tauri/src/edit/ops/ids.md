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

## next_text_id

```rust
pub(crate) fn next_text_id(doc: &EditDoc) -> String
```

`"t<n>"` for `doc.texts`.

## next_clip_id

```rust
pub(crate) fn next_clip_id(doc: &EditDoc) -> String
```

`"cl<n>"` for `doc.clips`. The prefix is `cl`, not `c` - `c` is already taken twice over, by `next_cut_id` (`timeops.rs`) and `next_caption_id` above, and a clip sharing either prefix would let a collision on `c<n>` silently steal an id that `next_clip_id`'s own highest-suffix scan would never see, since it only ever looks at `doc.clips`.

### Used by

- `src-tauri/src/edit/ops/api.rs` - the `AddLayoutSeg` / `AddCameraMove` arms.
- `src-tauri/src/edit/ops/motion.rs` - `add_zoom` (the shared `AddZoom` / `AddZoomFull` body).
- `src-tauri/src/edit/ops/captions.rs` - `apply_caption`'s `SplitCaption` arm.
- `src-tauri/src/edit/ops/textops.rs` - `apply_text`'s `AddText` arm.
- `src-tauri/src/edit/ops/clipops.rs` - `split`'s no-clips-yet branch (two new ids) and its splitting-an-existing-clip branch (one new id).
