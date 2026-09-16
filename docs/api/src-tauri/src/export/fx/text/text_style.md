# src-tauri/src/export/fx/text/text_style.rs

The four looks a text item can take, as a fixed table.

A text item stores its style as a plain string (`TextItem.style`), not an enum, because the ops layer already coerces unknown names (`edit::ops::textops::valid_text_style`) and because a string survives a document written by a newer build. This module is the one place that turns that string into pixels-worth of decisions: what colour the glyphs are, whether they carry a shadow, whether a scrim sits behind them, and whether a rule sits beside them. `textlayout` reads it to size and place the block; `textdraw` reads it to paint.

There are deliberately only four, and they are not user-editable: the point of the feature is that a text item looks considered without the user making typographic decisions (spec 5). A fifth look is a code change here plus a row in `src/editor/panels/textStyles.ts`, not a settings surface.

## Fill

```rust
pub enum Fill { White, Accent }
```

Where the glyph colour comes from, kept as a choice rather than a colour so that the accent is resolved at DRAW time.

*Why not store the colour:* there is exactly one accent in a document (`Settings.ui.accent`), and M5's ADDED-4 ruled that captions read it at draw time rather than copying it onto each item. Text follows the same ruling, so re-theming a project re-colours every accent text item at once and no item can carry a stale copy of a colour the user has since changed.

## TextStyle

```rust
pub struct TextStyle { pub fill: Fill, pub shadow: bool, pub plate: bool,
                       pub plate_rgb: [u8; 3], pub plate_alpha: f32, pub rule: bool }
```

One row of the table: everything the layout and the blit need to know about a look, and nothing about the item it is applied to.

- `fill` - the glyph colour, resolved through `fill_rgb`.
- `shadow` - whether the runs get `glyph::draw_run`'s 1 px dark shadow. On for the two styles that sit directly on the picture, off for the two that bring their own background or their own weight.
- `plate` - whether a rounded scrim sits behind the block. When true the layout reserves padding for it (`PAD_X`, `PAD_Y`) and the blit fills it first.
- `plate_rgb` and `plate_alpha` - that scrim's colour and its opacity, multiplied by the item's own animation alpha at draw time so the plate fades in and out with the text rather than popping.
- `rule` - whether a vertical rule sits on the anchor's side of the block. When true the layout reserves `2 * oh * RULE_W` of block width so the run clears the rule by one rule width.

`PartialEq` but not `Eq`: `plate_alpha` is an `f32`. The derive is here only so the tests can say "an unknown name gives you exactly the clean row".

## style_of

```rust
pub fn style_of(name: &str) -> TextStyle
```

The table. Four names, and everything else.

| name | fill | shadow | plate | plate colour | plate alpha | rule |
|---|---|---|---|---|---|---|
| `clean` | white | yes | no | - | - | no |
| `plate` | white | no | yes | black `[0, 0, 0]` | 0.62 | no |
| `accent` | the document accent | yes | no | - | - | no |
| `bar` | white | no | no | - | - | yes |

*Why `plate` drops the shadow:* the scrim already separates the glyphs from the picture, and a shadow on top of it reads as a smudge against the plate's own edge rather than as depth. *Why `bar` drops it too:* the rule is doing the separating, and the style's whole character is flatness.

*Why 0.62 and not "about 60 percent":* it is the same value `captiondraw`'s pill defaults to (`CaptionStyle::pill_alpha = 62`), so a text plate and a caption pill sitting in the same frame are the same darkness. One number, two features.

### Returns

A `TextStyle`. **Anything unknown returns the `clean` row**, including the empty string. That mirrors `edit::ops::textops::valid_text_style`, which coerces an unknown name to `"clean"` on the way into the document: the coercion happens at the op, and this fallback is the second line of defence for a document that reached the renderer some other way (a hand-edited `edit.json`, a future build's style name). Degrading to a readable default is always better than drawing nothing, because a text item that vanishes looks like a bug in the export rather than an unrecognised look.

### Behaviors

- `the_four_styles_are_the_table_the_spec_fixes` - each of the four rows asserted field by field, including the plate's black at 0.62 and the accent fill resolving to the accent that was passed in.
- `an_unknown_style_is_clean_rather_than_nothing` - `"wobble"` and `""` both come back exactly equal to `style_of("clean")`.
- `every_name_the_ops_module_accepts_has_a_row_here` - walks `edit::ops::textops::TEXT_STYLES`, pins its length at 4, and checks the two rows that are not the fallback are distinguishable, so adding a name to the ops list without adding a row here fails the build's tests rather than silently drawing it clean.

## fill_rgb

```rust
pub fn fill_rgb(f: Fill, accent: [u8; 3]) -> [u8; 3]
```

Resolve a `Fill` against the document's accent.

### Inputs

- `f: Fill` - the style's choice.
- `accent: [u8; 3]` - `Settings.ui.accent`, RGB. *Why passed in rather than read here:* the renderer owns reading settings (ADDED-4), which keeps this module a pure table with no lookups and makes every test able to name its own accent.

### Returns

`[255, 255, 255]` for `Fill::White`, `accent` verbatim for `Fill::Accent`. RGB order, which is what `glyph::put` expects and reverses into the frame's BGRA bytes.
