# src/editor/panels/textStyles.ts

The four text looks, mirrored from `src-tauri/src/export/fx/text/text_style.rs`, plus the option tables the Text surfaces pick from.

It sits in `panels/` rather than in `stage/text/` because the Add pills and the inspector dropdowns are its other readers and it is the one place a fifth look would be added; the preview layout imports `styleOf` from here rather than keeping a second copy, so there is exactly one table on the TypeScript side.

## TextStyle

```ts
export interface TextStyle {
  fill: "white" | "accent";
  shadow: boolean;
  plate: boolean;
  plateRgb: [number, number, number];
  plateAlpha: number;
  rule: boolean;
}
```

One row: everything the layout and the painter need about a look, and nothing about the item it is applied to. The camelCase twin of the Rust `TextStyle`.

`fill` is a CHOICE rather than a colour because there is exactly one accent in a document (`Settings.ui.accent`) and it is resolved at draw time (ADDED-4), so re-theming a project re-colours every accent text item at once and no item can carry a stale copy.

## TEXT_STYLES

```ts
export const TEXT_STYLES: Record<string, TextStyle>;
```

The table, keyed by the wire name a `TextItem.style` carries.

| name | fill | shadow | plate | plate colour | plate alpha | rule |
|---|---|---|---|---|---|---|
| `clean` | white | yes | no | - | - | no |
| `plate` | white | no | yes | black | 0.62 | no |
| `accent` | the document accent | yes | no | - | - | no |
| `bar` | white | no | no | - | - | yes, in the accent |

The `bar` row's fill and its rule are DIFFERENT colours, which is the style's whole point (spec 5.4): white type with the project's colour beside it. `LaidText` therefore carries `fill` and `ruleRgb` as two separate resolved values.

`plate` and `bar` both drop the shadow: the scrim and the rule are already doing the separating, and a shadow on top of either reads as a smudge rather than as depth. 0.62 is the same value a caption pill defaults to, so a text plate and a caption pill in one frame are the same darkness.

## styleOf

```ts
export const styleOf: (name: string) => TextStyle;
```

Look a name up, falling back to `clean` for anything unknown - the same coercion `edit::ops::textops::valid_text_style` applies on the way INTO the document and `text_style.rs::style_of` applies on the way out of it. Degrading to a readable default is always better than drawing nothing, because a text item that vanishes reads as a bug in the export rather than as an unrecognised look.

## TEXT_STYLE_OPTIONS

```ts
export const TEXT_STYLE_OPTIONS: { value: string; label: string }[];
```

The Style dropdown's rows, in the table's own order, with the wire name as the value. A plain `Picker`, not a gallery of swatches: four looks do not earn a visual chooser, and the stage shows the result immediately (`classic-layout-kept-shell-vetoed`).

## TEXT_KIND_OPTIONS

```ts
export const TEXT_KIND_OPTIONS: { value: TextKind; label: string }[];
```

The Kind dropdown's rows. `kind` is what `add_text` SEEDS from (the text, the sub, the size, the anchor and the style each kind starts with) and it stays on the item afterwards as a label, so changing it later renames the inspector's header without re-seeding anything the user has since edited.

## TEXT_PILLS

```ts
export const TEXT_PILLS: { kind: TextKind; name: string; hint: string }[];
```

The four Add pills in the Effects panel, one per kind: name, one-line hint, and the kind the pill seeds. `EffectPills.tsx` pairs each with an icon and builds both the click handler (`onAddText(kind)`) and the drag payload (`text:<kind>`) from it, so a fifth kind is one row here plus one icon rather than a fifth copied block of JSX.

The names are the user's words rather than the wire's: "Big Stat" for `stat`, "Lower Third" for `lower_third`.
