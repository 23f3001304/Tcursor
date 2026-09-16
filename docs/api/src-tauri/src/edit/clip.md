# src-tauri/src/edit/clip.rs

The clip list (`Clip`) that reorders the recording into pieces, in OUTPUT ORDER (spec 6.2). `src_in_ms`/`src_out_ms` are CLIP time - the recording's own clock, the same clock `Cut` and `Speed` already use - not output time; `EditDoc.clips` only says which source ranges play and in what order. An EMPTY `EditDoc::clips` means "one clip, the whole trim-resolved recording", which is what every document written before clips existed reads as, and what `TimeMap::build` (a later task) will produce byte-identically for such a doc. `transition_in_ms` of `0` is a hard cut and is ignored on the first clip, which has no predecessor to dissolve from.

## Clip

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Clip {
    pub id: String,
    pub src_in_ms: u32,
    pub src_out_ms: u32,
    #[serde(default)] pub transition_in_ms: u32,
}
```

One piece of the recording. `EditDoc.clips` is a `Vec<Clip>` with `#[serde(default)]` for back-compat; the list's array order IS the output order the clips play in.

- `id` - *stable string key, matching every other region's `id` convention. A re-record may renumber a clip's DISPLAY name, but the id still resolves to the same source range.*
- `src_in_ms` / `src_out_ms` - *this clip's range in CLIP time (the recording's own clock, not output time).*
- `transition_in_ms` - *cross-dissolve INTO this clip from the previous one, in ms. `0` - the default, and what every split produces - is a hard cut. Ignored on the first clip.*

### Used by

- `src-tauri/src/edit/model.rs` - `EditDoc.clips: Vec<Clip>`
- `src/shared/editClips.ts` - `Clip` TS mirror

### Behaviors

- `a_clip_without_a_transition_parses_as_a_hard_cut` - JSON with no `transition_in_ms` key deserializes it as `0`.
- `clips_round_trip_in_output_order` - two `Clip`s survive a full `EditDoc` JSON round trip in the array order they were written.
