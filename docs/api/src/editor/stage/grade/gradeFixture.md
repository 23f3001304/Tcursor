# src/editor/stage/grade/gradeFixture.ts

Test-only fixtures for the grade parity table, in the shape `src/editor/shell/shellFixture.tsx` already established: a plain module rather than exports from a `.test.ts`, because two test files need the same evaluator and importing one test file from another would register its suites twice.

It exists because the parity pin outgrew one file. `grade.test.ts` holds the 126 generated rows and the two tests that walk them; `gradeParams.test.ts` holds the behaviour of `paramsOf`, `seedOf` and `vignetteK`. The inputs both sides evaluate against live here.

## COLOURS

```ts
export const COLOURS: [number, number, number][]
```

The twelve input colours of spec 3.5, in the order the generated table is keyed by: pure black, pure white, mid grey, the three primaries, the three secondaries, a skin tone `(224, 172, 148)`, a dark blue `(18, 24, 48)`, and the app accent `(239, 68, 68)`. The same twelve, in the same order, as `COLOURS` in `export/grade/grade_table_tests.rs`. Reordering this array invalidates every row of `CENTRE`.

## PRESETS

```ts
export const PRESETS: GradePreset[]
```

The nine presets in table order. The generated rows are preset major and colour minor, so row `i` is `PRESETS[floor(i / 12)]` on `COLOURS[i % 12]`. Mirrors `PRESETS` in `grade_table_tests.rs`.

## IDENTITY

```ts
export const IDENTITY: GradeParams
```

The eleven parameters that change nothing: zero exposure, unit contrast and gamma and gain, no vignette, full saturation, no temperature or tint, no lift. `paramsOf` returns `null` rather than this value for an ungraded document, which is the point of the `null`; the table still needs a `GradeParams` to evaluate the `none` rows with, so `graded` substitutes this. Rust's `graded` helper does exactly the same thing with the same literal.

## graded

```ts
export function graded(p: GradePreset, c: [number, number, number], u: number, v: number)
```

Evaluates one table cell: seed the three stored numbers from the preset, resolve the eleven parameters (falling back to `IDENTITY` for the identity), run `gradePixel`, and round back to bytes. A line-for-line twin of the `graded` helper in `export/grade/grade_table_tests.rs`, because the whole point of the table is that the two produce the same numbers from the same inputs; the rounding in particular has to be the same, `Math.round` against Rust's `f32::round`, both half-away-from-zero on the non-negative values the clamp guarantees.
