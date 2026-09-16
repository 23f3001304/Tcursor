# src/editor/stage/grade/gradeParams.ts

The TypeScript twin of `src-tauri/src/export/grade/mod.rs`: the nine-row preset table of spec 3.2, the five named constants the formula uses, and the resolution from a document's `GradeSettings` to the eleven `GradeParams` the transform takes.

**It is the single source of those numbers for the whole preview.** `gradeCpu.ts` and `gradeShader.ts` both read the constants from here rather than restating them, which is the only thing keeping the luma weights and the vignette inset from drifting between the CPU transcription and the GLSL one; `gradeShader.test.ts` pins that the shader source still carries these exact literals.

Nothing here draws. The preview's drawing is `gradePass.ts`, and the export's is Rust.

## LUMA

```ts
export const LUMA: [number, number, number] = [0.2126, 0.7152, 0.0722];
```

The Rec.709 luma weights, RGB order, used by the saturation step. Mirrors Rust's `LUMA_R` / `LUMA_G` / `LUMA_B`, one array here because TypeScript has no reason to split them.

## VIGN_IN

```ts
export const VIGN_IN = 0.45;
```

The vignette's inner radius as a fraction of the corner distance; inside it the vignette is exactly zero. Mirrors Rust's `VIGN_IN`.

## CORNER

```ts
export const CORNER = 0.70710678;
```

`sqrt(2) / 2`, the centre-to-corner distance in the `u`/`v` space the formula works in. Dividing by it is what makes a vignette reach exactly 1 at a corner on any aspect ratio. Mirrors Rust's `CORNER`, to the same eight digits, because the parity table is pinned to one byte and a shorter constant would drift the corner rows.

## TEMP_GAIN

```ts
export const TEMP_GAIN = 0.25;
```

How far a temperature of +/-1 pushes red and blue apart. Mirrors Rust's `TEMP_GAIN`.

## TINT_GAIN

```ts
export const TINT_GAIN = 0.2;
```

How far a tint of +/-1 pushes green. Mirrors Rust's `TINT_GAIN`. Written `0.2` rather than `0.20` because that is what `String(0.2)` produces, and `gradeShader.test.ts` compares the shader source against exactly that string.

## GradeParams

```ts
export interface GradeParams {
  exposure: number;
  contrast: number;
  vignette: number;
  saturation: number;
  temp: number;
  tint: number;
  lift: [number, number, number];
  gamma: [number, number, number];
  gain: [number, number, number];
}
```

The eleven parameters, field for field the Rust `GradeParams`. The first three come from the document and the user can bend them; the other eight are the preset's own and are never stored. The three triples are fixed-length tuples rather than `number[]` so a row with the wrong arity is a type error rather than an undefined channel at run time.

## seedOf

```ts
export function seedOf(p: GradePreset): [number, number, number]
```

The `exposure`, `contrast` and `vignette` a pick WRITES into the document. Mirrors Rust's `seed_of`, and is what `GradeSection`'s Look picker calls: choosing a look sets all four document fields in one save, so it is one undo step and the picker keeps reading that look until the user bends a number away from this row.

Falls back to the `none` row for a preset name the table does not know, which is what makes a document written by a newer build load rather than crash on an unrecognised look.

## isIdentityGrade

```ts
export function isIdentityGrade(s: GradeSettings): boolean
```

True when all four fields are still the default. The TS twin of Rust's `GradeSettings::is_identity`, and the thing that has to agree with it: if the two ever disagreed, the preview would grade a frame the export leaves alone, or the reverse.

All four fields are checked, not just `preset`, so a bent knob over the `none` look still counts as a grade.

## paramsOf

```ts
export function paramsOf(s: GradeSettings): GradeParams | null
```

Resolves a document's grade into the eleven parameters, or `null` when there is no look. Mirrors Rust's `params_of`, including the `null` / `None` result, which is how `gradeCanvas` knows to return `false` and skip a WebGL pass altogether for the overwhelmingly common ungraded project.

Like `seedOf` it falls back to the `none` row for an unknown preset name.

### Used by

- `src/editor/stage/grade/gradePass.ts` (`gradeCanvas`) - resolves the settings before touching WebGL
- `src/editor/stage/grade/gradeFixture.ts` (`graded`) - the parity table's evaluator

## vignetteK

```ts
export function vignetteK(u: number, v: number): number
```

The vignette's radial falloff, 0 at the centre of the frame and 1 at a corner. `u` and `v` are the pixel-centre offsets `(x + 0.5) / ow - 0.5` and `(y + 0.5) / oh - 0.5`. Mirrors Rust's `vignette_k`; `gradePixel` squares the result before scaling by `vignette`.

### Behaviors

- `returns null for the identity and something for a bent knob` - `paramsOf` on the default settings is `null`; an exposure of 0.05 over preset `none` is not.
- `seeds the three stored numbers from the preset row` - `seedOf` against spec 3.2 for `none`, `cinematic` and `midnight`.
- `lets the stored numbers win while the other eight stay the preset's` - a Noir with all three stored numbers bent keeps Noir's saturation of zero.
- `puts the vignette at zero in the centre and one at a corner` - the two endpoints of `vignetteK`, exact.
