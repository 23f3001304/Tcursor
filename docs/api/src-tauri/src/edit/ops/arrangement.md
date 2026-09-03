# src-tauri/src/edit/ops/arrangement.rs

Arrangement edit ops (T34), split out of `api.rs` so each file stays under the size limit; `api::apply` delegates the `SetArrangement`/`ClearArrangement` variants here. Pure doc mutation - no I/O. Both ops ride the one serialized load -> apply -> save that the `apply_edit_op` command already performs under `edit::lock::doc_lock`, so each is a single atomic doc write and therefore a single undo step, exactly like every other op.

## MIN_SIZE

```rust
const MIN_SIZE: f32 = 0.05;
```

Floor for `PanelPose.size` (the panel's height fraction). Below this a panel would be effectively invisible but still *shown*, which is a confusing state to be able to save - hiding a panel is a `None` pose, not a tiny one.

## MAX_SIZE

```rust
const MAX_SIZE: f32 = 1.5;
```

Ceiling for `PanelPose.size`. Above 1.0 the panel deliberately overflows the frame (a legitimate bleed-off-frame composition); 1.5 is where growing it further stops changing what is visible.

## clamp_pose

```rust
pub(crate) fn clamp_pose(p: PanelPose) -> PanelPose
```

Clamp a pose into the addressable range: `cx`/`cy` to `0..1`, `size` to `MIN_SIZE..MAX_SIZE`. Applied on the way IN, so the doc never stores a pose that resolution would have to defend against.

Note `cx`/`cy` bound the panel's CENTER, not its edges - a panel may still overhang the frame, which is intended (a half-off-frame webcam is a real composition). What the clamp prevents is a center off the frame entirely, where the panel would be invisible with no on-stage handle left to drag it back by.

## double_option

```rust
pub fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where D: Deserializer<'de>, T: Deserialize<'de>
```

`deserialize_with` for an `Option<Option<T>>` field that must tell an ABSENT key ("leave this panel as it is") apart from an explicit `null` ("hide this panel").

**Why it is needed:** plain `Option<Option<T>>` cannot express that distinction. Serde folds a `null` into the OUTER `Option`, producing `None` - the same value `#[serde(default)]` produces for a missing key - so "hide the cam" would arrive indistinguishable from "don't touch the cam". Deserializing the INNER `Option` (where `null` legitimately means `None`) and wrapping the result in `Some` keeps the two apart, with no new dependency (this is what `serde_with::double_option` does).

Paired with `skip_serializing_if = "Option::is_none"` on the same field, so the round trip is exact in both directions: an untouched panel is OMITTED rather than written as `null` (which would come back meaning "hide it"). Pinned by `the_wire_form_distinguishes_an_absent_key_from_an_explicit_null` and `an_untouched_panel_is_omitted_from_the_serialized_op`.

### Used by

- `src-tauri/src/edit/ops/api.rs` - both panel fields of `EditOp::SetArrangement`

## apply_arrangement

```rust
pub fn apply_arrangement(doc: &mut EditDoc, op: EditOp)
```

Apply one arrangement op in place; a no-op for anything else (the match arm in `api::apply` only routes the two arrangement variants here).

### Inputs (what, and why it is needed)

- `doc: &mut EditDoc` - the loaded document. *Why:* the op is addressed by segment id and mutates `doc.layout` in place, so the caller's single save persists it.
- `op: EditOp` - `SetArrangement` or `ClearArrangement`. *Why:* taking the whole enum (rather than the destructured fields) is the same delegation shape `effects::apply_effect` uses, so `api::apply` stays a flat dispatch table.

### Semantics

`SetArrangement` is a PARTIAL update over the segment's current arrangement, three-valued per panel:

- field absent (`None`) - *leave that panel exactly as it is.*
- field `null` (`Some(None)`) - *hide that panel.*
- field a pose (`Some(Some(p))`) - *set it, through `clamp_pose`, and un-hide it.*

On a segment that has NO arrangement yet the base is `Arrangement { screen: None, cam: None }`, so a panel the op does not touch starts hidden. This is deliberate rather than a gap: converting a preset means sending BOTH panels, and the frontend always has both - they arrive on `LayoutPresetDto.arrangement` from `preview_layouts`, derived by `scene::arrangement::arrangement_of_preset`. A one-panel first set is therefore a genuine "just this panel" arrangement, never a half-initialised one.

**The at-least-one-panel clamp.** A change that would leave both panels hidden is REJECTED - the doc is left untouched, including a bare segment which stays on its preset - rather than saved as an arrangement that renders an empty frame. `ClearArrangement` is the way back to the preset.

`ClearArrangement` sets `arrangement` back to `None`, so the segment resolves from its `layout` preset again. Idempotent, and harmless on a segment that never had one. An unknown segment id is a no-op for both ops (the frontend can race a segment removal against a drag).

### Used by

- `src-tauri/src/edit/ops/api.rs` - `apply` delegates the two arrangement variants here
