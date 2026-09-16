# src-tauri/src/edit/text.rs

Animated text overlays (`TextKind`, `TextAnchor`, `TextSize`, `TextAnim`, `TextItem`) for titles, lower thirds, stats and typewriter callouts (spec 5.1). Unlike a mask's `rect` (`effect.md`), a `TextItem` does NOT live in canvas-fraction space: text is an overlay on the OUTPUT frame, and every fraction on it (`offset`, the size rungs) is a fraction of the output frame, not the recorded canvas. `texts` carries no `layer` field - overlap is a display concern for `layoutRegions`, the same way `Speed` has none - so draw order is array order, which is creation order. A `"accent"`-style item's colour is read from `Settings.ui.accent` at render time and is never stored on the item itself, the same ruling M5 made for captions.

## TEXT_SIZE_FRACS

```rust
pub const TEXT_SIZE_FRACS: [f32; 5] = [0.030, 0.042, 0.058, 0.082, 0.115];
```

The five `TextSize` rungs, as fractions of OUTPUT HEIGHT, indexed by the enum's own discriminant (`Xs` = 0 ... `Xl` = 4). See `TextSize::frac`.

## TextKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextKind { #[default] Title, LowerThird, Stat, Callout }
```

What a text item represents. Serializes snake_case (`"title"` / `"lower_third"` / `"stat"` / `"callout"`) to match the TS `TextKind`. Purely descriptive on this type - it changes nothing about layout or rendering by itself; `AddText`'s seed values (`edit::ops::textops`, a later task) key off it to pick starting content, position and style per kind.

## TextAnchor

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextAnchor {
    TopLeft, TopCenter, TopRight,
    MidLeft, MidCenter, MidRight,
    BottomLeft, #[default] BottomCenter, BottomRight,
}
```

One of nine positions on the output frame. `TopLeft` is the frame's own top left corner; `TextItem.offset` is added after the anchor resolves, so a nudge stays relative to whichever corner or edge the item is pinned to. Serializes snake_case (`"top_left"`, ... `"bottom_right"`) to match the TS `TextAnchor`.

## TextSize

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextSize { Xs, S, #[default] M, L, Xl }
```

One of five height rungs. Serializes lowercase (`"xs"` / `"s"` / `"m"` / `"l"` / `"xl"`) to match the TS `TextSize`. The discriminant IS the index into `TEXT_SIZE_FRACS`; see `TextSize::frac`.

## TextSize::frac

```rust
pub fn frac(self) -> f32
```

This rung's font height, as a fraction of output height: `TEXT_SIZE_FRACS[self as usize]`. `TextSize::M.frac()` is `0.058`.

## TextAnim

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextAnim { #[default] Fade, Slide, Pop, Typewriter }
```

One of four in/out animation styles, chosen independently for `TextItem.anim_in` and `TextItem.anim_out`. Serializes lowercase (`"fade"` / `"slide"` / `"pop"` / `"typewriter"`) to match the TS `TextAnim`. The layout math each one drives is spec 5.4 (`export::fx::textlayout`, a later task).

## TextItem

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextItem {
    pub id: String,
    pub start_ms: u32, pub end_ms: u32,
    pub kind: TextKind,
    pub text: String,
    pub sub: Option<String>,
    pub style: String,
    pub pos: TextAnchor,
    pub offset: [f32; 2],
    pub size: TextSize,
    pub anim_in: TextAnim, pub anim_out: TextAnim,
    pub in_ms: u32, pub out_ms: u32,
    pub easing: String,
}
```

One animated text overlay on the timeline. `EditDoc.texts` is a `Vec<TextItem>` with `#[serde(default)]` for back-compat.

- `id` - *stable string key, matching every other region's `id` convention (`Zoom.id`, `Cut.id`, ...).*
- `start_ms` / `end_ms` - *the item's span, OUTPUT-clock ms; moved by `remap_doc`.*
- `kind` - *what the item represents; defaults to `Title`.*
- `text` - *the main line.*
- `sub` - *the optional second line: a lower third's role, a stat's caption, nothing on a title. Absent is not written (`skip_serializing_if`). The double-optional wire behaviour for EDITING it (absent leaves it, `null` clears it, a string sets it) belongs to `edit::ops::textops`, not to this type.*
- `style` - *a style preset name (`export::fx::text_style::STYLES`, a later task); anything unknown falls back to `"clean"`. Defaults to `"clean"` (`default_text_style`).*
- `pos` - *which of the nine `TextAnchor`s the item is pinned to; defaults to `BottomCenter`.*
- `offset` - *added to the anchor's resolved position, as fractions of OUTPUT width/height, `-0.5..0.5`; defaults to `[0.0, 0.0]`.*
- `size` - *which `TextSize` rung; defaults to `M`.*
- `anim_in` / `anim_out` - *the entry and exit animation; both default to `Fade`.*
- `in_ms` / `out_ms` - *duration of the entry/exit animation, ms; both default to `420` (`default_anim_ms`).*
- `easing` - *an M3 easing string - a named curve, `cubic(..)`, `spring(..)` or `keys(..)` - coerced on the way in by `edit::ops::region::valid_easing` (a later task), the same gate every other easing field goes through. Defaults to `"smooth"` (`default_text_easing`).*

### Used by

- `src-tauri/src/edit/model.rs` - `EditDoc.texts: Vec<TextItem>`
- `src-tauri/src/edit/remap_doc.rs` - moved onto the output clock, like every other region list
- `src/shared/editText.ts` - `TextItem` TS mirror

### Behaviors

- `a_doc_written_before_texts_existed_loads_with_an_empty_list` - a v2 doc with no `texts` (or `clips`) key parses and both are empty; adding the fields is not a schema bump.
- `a_text_with_only_the_required_fields_takes_the_spec_defaults` - JSON with only `id`/`start_ms`/`end_ms`/`text` fills every other field from its spec default.
- `the_enums_are_snake_and_lower_case_on_the_wire` - `TextKind` and `TextAnchor` serialize snake_case; `TextSize` and `TextAnim` serialize lowercase.
- `a_full_text_round_trips_through_a_doc_and_absent_sub_is_not_written` - a fully populated `TextItem` survives a full `EditDoc` JSON round trip; one with `sub: None` serializes with no `"sub"` key at all.
- `the_size_rungs_are_the_spec_fractions_of_output_height` - pins `TEXT_SIZE_FRACS` to the spec's five numbers and `TextSize::M.frac()` to `0.058`.

## default_text_style

```rust
pub fn default_text_style() -> String
```

Serde default for `TextItem.style`: `"clean"`.

## default_anim_ms

```rust
pub fn default_anim_ms() -> u32
```

Serde default for `TextItem.in_ms` and `TextItem.out_ms`: `420`.

## default_text_easing

```rust
pub fn default_text_easing() -> String
```

Serde default for `TextItem.easing`: `"smooth"`.
