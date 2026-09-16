# src/editor/stage/grade/gradeShader.ts

The preview's grade as GLSL ES 3.0, built as a template string over the constants `gradeParams.ts` exports. Two sources: a full-screen triangle vertex shader with no attributes, and the fragment shader that is a transcription of Rust's `export::grade::apply_px`.

**Why the constants are interpolated rather than written out.** There are now four transcriptions of the same eight steps (Rust, WGSL, this GLSL, and `gradeCpu.ts`), and the numbers are the part most likely to drift silently: nobody notices a `0.7071` where `0.70710678` belongs until a corner pixel is one byte off. Interpolating them means the GLSL and the TypeScript literally cannot disagree, and `gradeShader.test.ts` pins that they are still present in the emitted source, so deleting a `const` line from the shader fails a test rather than falling back to a default.

## VERTEX_SOURCE

```ts
export const VERTEX_SOURCE: string
```

A full-screen triangle from `gl_VertexID` alone, with no attribute buffer, no vertex array object and no `bufferData`. `(gl_VertexID << 1) & 2` and `gl_VertexID & 2` give `(0,0)`, `(2,0)`, `(0,2)`, which map to clip-space `(-1,-1)`, `(3,-1)`, `(-1,3)`: a triangle large enough that its intersection with the viewport is the whole screen. One triangle rather than two so there is no diagonal seam where the halves meet, and no buffer at all so `build()` has nothing to allocate or free.

## gradeFragmentSource

```ts
export function gradeFragmentSource(): string
```

The fragment shader. Samples the 2D canvas bound as `tex`, applies the eight steps of spec 3.3 in the fixed order, and writes the result with the source alpha untouched.

**Uniform layout, mirroring `pack_grade` slot for slot** so the two packings can be read side by side: `gA` is `[exposure, contrast, saturation, vignette]`, `gB` is `[temp, tint, active, pad]`, then `lift`, `gam` and `gain` as `vec3`. The `active` flag is uploaded but not branched on here, because `gradeCanvas` has already returned `false` for an ungraded project and the shader never runs at all in that case; on the GPU export path there is one shader for everything, so the flag has to exist there.

**The texture flip.** `gl_FragCoord` has its origin at the bottom left, while a 2D canvas uploaded with `texImage2D` has its first row at the top, so the sample coordinate is `vec2(px.x / dims.x, 1.0 - px.y / dims.y)`. The vignette's own `px / dims - 0.5` is NOT flipped, and does not need to be: the falloff is radially symmetric about the centre, so mirroring y changes nothing. Flipping one and not the other is deliberate and the reason they are written as two separate expressions.

**`contrast` and `vignette` exist as named locals** that the arithmetic would not otherwise need. They give `gradeShader.test.ts`'s order check stable tokens to search for, and they are what a reader scanning for a step looks for.

### Behaviors

- `declares GLSL ES 3.0 and one output` - the `#version 300 es` line comes first (it must be the very first line or the shader will not compile) and there is exactly one `out vec4`.
- `carries the same constants gradeParams.ts does, so they cannot drift` - all seven of `LUMA` (three), `VIGN_IN`, `CORNER`, `TEMP_GAIN` and `TINT_GAIN` appear in the emitted source, compared as `String(n)` so a change of precision on either side fails.
- `keeps the eight steps in the order the Rust reference fixes` - `exp2`, `lift`, `pow`, `contrast`, `luma`, `vignette` appear in that order in the source. A weak check, deliberately: it cannot see a wrong coefficient, but it does catch the one failure mode that matters most here, a step moved or removed by someone editing the shader without the Rust reference open.
