# src/shared/editText.ts

TypeScript mirror of `src-tauri/src/edit/text.rs`'s `TextItem` and its four enums, re-exported from `src/shared/edit.ts`. Keep in sync with the Rust side.

## TextKind

```ts
export type TextKind = "title" | "lower_third" | "stat" | "callout";
```

What a text item represents - mirrors Rust `TextKind`. Purely descriptive by itself; `add_text`'s seed values (a later task) key off it to pick starting content, position and style per kind.

## TextAnchor

```ts
export type TextAnchor =
  | "top_left" | "top_center" | "top_right"
  | "mid_left" | "mid_center" | "mid_right"
  | "bottom_left" | "bottom_center" | "bottom_right";
```

One of nine positions on the output frame - mirrors Rust `TextAnchor`. `"top_left"` is the frame's own top left corner; `TextItem.offset` is added after the anchor resolves.

## TextSize

```ts
export type TextSize = "xs" | "s" | "m" | "l" | "xl";
```

One of five height rungs - mirrors Rust `TextSize`. See `TEXT_SIZE_FRACS`.

## TextAnim

```ts
export type TextAnim = "fade" | "slide" | "pop" | "typewriter";
```

One of four in/out animation styles - mirrors Rust `TextAnim`, chosen independently for `TextItem.anim_in` and `TextItem.anim_out`.

## TEXT_SIZE_FRACS

```ts
export const TEXT_SIZE_FRACS: Record<TextSize, number> = { xs: 0.03, s: 0.042, m: 0.058, l: 0.082, xl: 0.115 };
```

The five `TextSize` rungs, as fractions of OUTPUT HEIGHT - mirrors Rust `TEXT_SIZE_FRACS`, keyed by name here rather than by discriminant index.

## TextItem

```ts
export interface TextItem {
  id: string;
  start_ms: number; end_ms: number;
  kind: TextKind;
  text: string;
  sub?: string | null;
  style: string;
  pos: TextAnchor;
  offset: [number, number];
  size: TextSize;
  anim_in: TextAnim; anim_out: TextAnim;
  in_ms: number; out_ms: number;
  easing: string;
}
```

One animated text overlay on the timeline - mirrors Rust `TextItem`. Held in `EditDoc.texts`.

- `sub?: string | null` - *the optional second line. Rust omits the key entirely when unset (`skip_serializing_if`); the double-optional wire behaviour for EDITING it belongs to the `update_text` op (a later task), not to this type.*
- `offset: [number, number]` - *added to the anchor's resolved position, as fractions of output width/height, `-0.5..0.5`.*
- `in_ms` / `out_ms: number` - *duration of the entry/exit animation, ms.*
- `easing: string` - *an M3 easing string, same free-form convention as `Zoom.easing`.*

### Used by

- `src/shared/edit.ts` - field `EditDoc.texts`, re-exports this type.
