# src/editor/stage/grade/gradeCpu.ts

A literal transcription of Rust's `export::grade::apply_px` into TypeScript. **It never draws a pixel.** The preview grades on the GPU (`gradePass.ts`), and the export grades in Rust; this file exists only so that the two can be pinned to each other on the CPU, where a test can read the numbers.

That is worth one file because of what the alternative would be. The preview's real transform is GLSL inside a template string, which no unit test can evaluate without a WebGL2 context, and vitest's environment has none. So the shader is guarded structurally (`gradeShader.test.ts` pins that it carries the same constants in the same order) while the arithmetic is guarded here, against the same 126 rows the Rust side generated. Between them, a change to the formula that is made on one side and not the others fails a test rather than shipping a preview that quietly disagrees with the export.

## gradePixel

```ts
export function gradePixel(
  c: [number, number, number],
  p: GradeParams,
  u: number,
  v: number,
): [number, number, number]
```

The eight steps of spec 3.3 on one pixel. `c` is `[R, G, B]` in 0..1, sRGB-encoded in value with no transfer function applied anywhere; `u` and `v` are the pixel-centre offsets from the frame centre, `(x + 0.5) / ow - 0.5` and `(y + 0.5) / oh - 0.5`. The return is clamped to 0..1.

Step for step it is `apply_px`: exposure and the temperature and tint channel gains in one expression, then lift and gain with the clamp before `pow` (without it a lifted, exposed highlight goes negative and `Math.pow` returns `NaN`), then gamma, contrast about a 0.5 pivot, Rec.709 saturation, and the squared radial vignette. The constants all come from `gradeParams.ts` so they cannot drift from the shader's.

**Do not "improve" this function.** Reordering the steps, folding two of them together or hoisting an invariant changes the bytes, and the bytes are pinned on both sides of a language boundary. If the formula has to change, it changes in `export/grade/mod.rs` first, the Rust generators are re-run, and the new rows are pasted here and there in the same change.

**The tolerance is one byte, deliberately.** Rust evaluates the formula in `f32` and JavaScript in `f64`, so the two round the same expression differently in the last place. Pinning to exact equality would fail on arithmetic that is correct; pinning to one byte catches every real divergence, because any actual change to the formula moves a channel by far more than that.

### Used by

- `src/editor/stage/grade/gradeFixture.ts` (`graded`) - the only caller, and it is test-only

### Behaviors

- `matches at the frame centre for every preset and colour, within one byte` - the 108 rows `export/grade/grade_table_tests.rs` generated, nine presets across twelve colours, preset major and colour minor.
- `matches at an edge midpoint and a corner for the two heaviest vignettes` - the other 18 generated rows, which are the ones that exercise the vignette at all.
