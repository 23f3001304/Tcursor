# src-tauri/src/edit/ops/textops.rs

The animated-text track's edit ops (`AddText`, `UpdateText`, `RemoveText`), split out of `api.rs` the way `effects.rs` and `captions.rs` are. `api::apply` routes all three variants here in one delegating arm. Like every other region list it clamps and orders through `region::{clamp_order, dur_bound}`; unlike `captions`/`camera_moves` it is never re-sorted after a mutation, because `texts` carries no `layer` and draws in array order, which is creation order (`text.md`). Nothing renders `EditDoc.texts` yet (spec 5.2) - this module only maintains it.

## TEXT_STYLES

```rust
pub const TEXT_STYLES: [&str; 4] = ["clean", "plate", "accent", "bar"];
```

The four style preset names a `TextItem.style` may hold (`export::fx::text_style::STYLES`, a later task, renders them). Checked by `valid_text_style`.

## MAX_TEXT_CHARS

```rust
pub const MAX_TEXT_CHARS: usize = 200;
```

The longest `text` or `sub` an item may store, in characters (`chars().count()`, not bytes). Enforced by the private `tidy` helper, which every `text`/`sub` write in `UpdateText` goes through.

## valid_text_style

```rust
pub fn valid_text_style(s: &str) -> String
```

Coerces a style wire-name to one of `TEXT_STYLES`; anything else becomes `"clean"` - shaped exactly like `region::valid_layout`, and for the same reason: a typo'd or stale style name must never reach the renderer.

## apply_text

```rust
pub fn apply_text(doc: &mut EditDoc, op: EditOp)
```

Applies one text op:

- `AddText { at_ms, dur_ms, kind }` seeds a new `TextItem` by `kind` (the table below), pushes it with a generated id (`ids::next_text_id`: `t0`, `t1`, ...), and clamps its span into `[0, region::dur_bound(doc)]`, the same as every other `Add*` op. `anim_out` always starts `Fade`, and `in_ms`/`out_ms`/`easing` always start at the `TextItem` spec defaults (`420`/`420`/`"smooth"`) regardless of kind - only the table's own columns vary by kind.
- `UpdateText { id, .. }` patches only the fields it carries into the matching item; an unknown `id` is a no-op.
  - `start_ms`/`end_ms` clamped to `dur_bound(doc)`, then `region::clamp_order` so a partial update can never leave the item inverted.
  - `text` and `sub` trimmed and truncated to `MAX_TEXT_CHARS` characters. An all-whitespace `text` is stored as-is (trimming an all-space string yields `""`) and renders nothing, mirroring M5's empty-caption rule.
  - `sub` is double-optional on the wire, deserialized with the existing `edit::ops::arrangement::double_option` helper `SetArrangement` already uses: the key absent leaves it, `null` clears it to `None`, a string tidies and sets it.
  - `kind` overwrites outright - changing kind does not re-seed content, position or style; those are one-time `AddText` seeds, not a live binding.
  - `style` coerced by `valid_text_style`.
  - `pos`, `size`, `anim_in`, `anim_out` overwrite outright; each is already a closed enum, so there is nothing to validate.
  - `offset` clamped per component to `[-0.5, 0.5]`; a component that is `NaN` or infinite drops the WHOLE write (`Option::filter`), leaving the item's stored offset untouched rather than storing a partial or poisoned pair.
  - `in_ms`/`out_ms` clamped to `[0, 4000]` - a ceiling only, so `0` (an instant cut) is legal.
  - `easing` through `region::valid_easing`, the same gate every other easing field goes through.
- `RemoveText { id }` drops the item by id (`retain`); an unknown id is a no-op.

Any other op is a no-op (`_ => {}`) - `api::apply`'s match only routes the three text variants here, so the fallback is unreachable by contract, the same shape `effects::apply_effect` and `captions::apply_caption` use.

### AddText seed table

| kind | text | sub | size | pos | style | anim_in |
|---|---|---|---|---|---|---|
| `Title` | "Your title" | none | `L` | `MidCenter` | `clean` | `Fade` |
| `LowerThird` | "Name" | "Role" | `M` | `BottomLeft` | `bar` | `Fade` |
| `Stat` | "128" | "faster" | `Xl` | `MidCenter` | `clean` | `Fade` |
| `Callout` | "A callout" | none | `S` | `BottomCenter` | `plate` | `Typewriter` |

### Behaviors

- `an_added_text_lands_inside_the_clip` - a 2 s add starting 500 ms before a 10 s clip's end is clamped to `[9500, 10000]`; the id starts with `t`.
- `each_kind_seeds_its_own_text_size_anchor_and_style` - all four `AddText` seed rows (text, sub, size, pos, style, and `Callout`'s `anim_in`), plus the `t0`/`t1`/`t2`/`t3` id sequence.
- `update_clamps_the_span_and_orders_it` - moving `start_ms` past the stored `end_ms` pulls `end_ms` with it; a far-future `end_ms` clamps to the clip length.
- `the_double_optional_sub_leaves_clears_and_sets` - an absent `sub` key leaves the seeded value, `null` clears it, a string sets it.
- `text_is_trimmed_and_truncated_and_whitespace_is_kept_as_is` - a padded, over-length `text` is trimmed and cut to `MAX_TEXT_CHARS`; an all-whitespace `text` stores as `""`.
- `valid_text_style_falls_back_to_clean` - every `TEXT_STYLES` entry round-trips through `valid_text_style`; an unknown name becomes `"clean"`, both directly and through `UpdateText`.
- `offset_is_clamped_per_component_and_a_non_finite_offset_is_dropped` - `[0.9, -0.9]` clamps to `[0.5, -0.5]`; an offset with a `NaN` component leaves the stored offset unchanged.
- `animation_times_are_clamped_and_easing_is_coerced` - `in_ms: 9000` clamps to `4000`, `out_ms: 0` stays `0`; an unknown `easing` degrades to `"smooth"`; a well-formed `cubic(..)` canonicalises.
- `remove_text_drops_the_one_item_and_an_unknown_id_is_a_noop` - an unknown id changes nothing; a known id removes only that item.
