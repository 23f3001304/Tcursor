# src-tauri/src/edit/ops/motion.rs

The project's motion language (`settings::motion::MotionSettings`) applied to regions: what a NEWLY added zoom / layout segment / camera keyframe inherits, and the one-shot sweep that stamps it onto everything already placed. Split out of `api.rs` exactly the way `effects.rs` and `timeops.rs` are - `api.rs` was at 184 of its 200 lines before M3 and could not hold these bodies.

Everything here funnels through `region::valid_easing`, so a curve reaching the doc from the project default is canonicalised by the same rule as a curve reaching it from an inspector. There is no curve PARSING in this file: it carries strings.

## add_zoom

```rust
pub(crate) fn add_zoom(doc: &mut EditDoc, at_ms: u32, dur_ms: u32, scale: f32)
```

Places a NEW zoom: the id from `ids::next_zoom_id`, the span clamped to `region::dur_bound(doc)` (with a `saturating_add` against u32 overflow), the row from `region::auto_layer` over the zooms already placed, and the curves from `for_zoom` below. `target` is `Cursor`, the ramps 350/450 ms.

The body `EditOp::AddZoom` and `EditOp::AddZoomFull` share verbatim - they differ only in `scale` (2.0 vs the caller's). Moved here from `api.rs` in M5 T4 as a pure move, with no behaviour change, to make room for the six caption variants under that file's 200-line cap; it lives in `motion.rs` rather than a file of its own because the curves it stamps are exactly what this module is for.

## for_zoom

```rust
pub(crate) fn for_zoom(doc: &EditDoc) -> (String, Option<String>)
```

The `(easing, easing_out)` a new `Zoom` inherits.

### Returns

The project's in ramp, and its out ramp **only when the two genuinely differ**. When they agree the second element is `None`, which - with `Zoom::easing_out`'s `skip_serializing_if` - means a Soft / Snappy / Mechanical / Bouncy project writes exactly the shape of `Zoom` it wrote before the field existed: one curve, no new key on the wire. Only a split preset (Cinematic, today) ever puts a second curve in the JSON.

### Used by

- `api::apply` - `AddZoom` and `AddZoomFull`
- `apply_default` below

## for_layout

```rust
pub(crate) fn for_layout(doc: &EditDoc) -> (String, String)
```

The `(easing, easing_out)` a new `LayoutSeg` inherits. Both are always stored: `LayoutSeg.easing_out` has been a required field since exit transitions landed, so there is nothing to omit and no `Option` to collapse.

`AddLayoutSeg`'s own `easing_out` argument still WINS when the caller passes one (the AI director and the layout panel both do) - the project default is the fallback, not an override.

## for_camera

```rust
pub(crate) fn for_camera(doc: &EditDoc) -> String
```

The single `easing` a new `CameraMove` keyframe inherits: the ramp INTO the keyframe, so it is the project's IN curve. A camera move has no exit ramp of its own - the only thing that follows it is the next keyframe's entry - which is why `easing_out` has no meaning here and `CameraMove` keeps its one `easing` field.

## set_zoom_easing_out

```rust
pub(crate) fn set_zoom_easing_out(z: &mut Zoom, v: &str)
```

`UpdateZoom.easing_out`'s write, and the place the field's THREE-valued wire meaning is decided.

### The clearing semantics, and why

`UpdateZoom`'s convention is "field absent (`None`) = leave unchanged", which alone cannot express "clear the override back to inherit". The two ways out are `Option<Option<String>>` with a `double_option` deserializer (what `SetArrangement` does for its panel poses) or a sentinel inside the string. This op takes the **sentinel**: an EMPTY string clears `easing_out` back to `None` ("the same curve as `easing`"), any other string is canonicalised and stored.

*Why the sentinel here when `SetArrangement` went the other way:* an empty string is not a legal curve in the first place - `valid_easing("")` degrades it to `"smooth"` - so it can never collide with a value a client might legitimately mean, whereas a `PanelPose` has no such spare value. The sentinel also keeps the op ONE flat struct on the wire, so the TypeScript `EditOp` union stays a plain `easing_out?: string` instead of needing a null-versus-absent distinction that `JSON.stringify` is famously bad at preserving.

### Behaviors worth knowing

- A curve equal to the zoom's own `easing` **collapses to `None`** rather than being stored as a duplicate: the doc has exactly one way to say "both ramps are the same", so `presetOf` on the frontend and `regions_from_doc` on the backend never have to reconcile two spellings of it.
- Because of that collapse, a call that changes BOTH ramps must have `easing` applied first. `api::apply` does (the `easing` arm sits directly above the `easing_out` one), and that ordering is load-bearing, not incidental.
- `update_zoom_sets_clears_and_collapses_the_out_ramp` (`motion_tests.rs`) pins set, collapse, the empty-string clear, and "absent leaves it alone".

## apply_default

```rust
pub(crate) fn apply_default(doc: &mut EditDoc)
```

`EditOp::ApplyMotionDefault`: stamp `doc.settings.motion` onto EVERY zoom, layout segment and camera move at once - the editor's "Apply to all regions" button.

### Behavior

Only the CURVES change. No duration, target, pose, layer or span is touched, so a project keeps its timing exactly and gains one consistent feel. A zoom's `easing_out` is set from `for_zoom`, which means Apply also **un-splits** a region that had been given its own out curve whenever the project's two ramps agree.

One op, so the whole sweep is one undo step - the same reason `AddCuts` exists as a batch rather than a loop of `AddCut`.

### Behaviors

- `apply_motion_default_stamps_every_region_as_one_step` (`motion_tests.rs`) - three regions, each dragged onto its own curve, all three on the project's curve afterwards, with the zoom's split dropped and its `zoom_in_ms`/`zoom_out_ms` untouched.
- `the_default_project_still_writes_todays_smooth_everywhere` - the guard for every existing recording: an untouched project's add ops produce byte-identical regions to the ones they produced before `settings.motion` existed.
- `a_snappy_preset_string_survives_valid_easing` is `#[ignore]`d until the `keys(...)` arm of `valid_easing` lands (the Rust evaluator's task); the rest of this file's tests deliberately use `"linear"` / `"smooth"` / `spring(...)`, which are plumbing-equivalent and parse today.
