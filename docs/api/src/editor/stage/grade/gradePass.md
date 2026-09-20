# src/editor/stage/grade/gradePass.ts

The preview's colour grade: a lazy offscreen WebGL2 context, one texture and one full-screen triangle, copied back over the 2D stage canvas once per composited frame. About a hundred lines and no new package, because raw WebGL2 through `canvas.getContext("webgl2")` is a browser API.

**Why the preview grades itself instead of asking the backend.** Two reasons, both from spec 3.5. The backend FX overlay cannot carry a grade at all: it reconstructs straight alpha from one render over black and one over white, and a grade is a per-pixel remap rather than an over-composite, so there is no alpha that would reproduce it. And routing the whole frame through Rust per tick is out on cost: the FX request is already the app's hottest command at roughly 25 calls a second, each doing two GPU render-and-readback fences, a PNG deflate and a base64 encode, at HALF resolution precisely because the round trip is expensive. `export/preview/preview_fx.rs` is not touched by the grade at all.

**Where it lands in the frame.** Between `drawPreview` (background, screen panel, webcam panel) and `drawCursorLayer`, which is the seam Task 0a opened by splitting the cursor blit out of `drawPreview`. That is what makes the grade cover the webcam panel and not the cursor, matching the export (spec 1.3).

## gradeCanvas

```ts
export function gradeCanvas(
  ctx: CanvasRenderingContext2D,
  c: HTMLCanvasElement,
  s: GradeSettings,
): boolean
```

Grades the whole of `c` in place. Returns `true` when it drew, `false` when it did not.

It returns `false` in two cases, and both are normal:

- **There is no look.** `paramsOf(s)` is `null` for an ungraded document, which is most of them. This check comes first, before any WebGL object is touched, so a project with no grade never creates a context at all and pays one object-field comparison per frame.
- **WebGL2 is unavailable**, or the shader failed to compile or link. The failure is latched in a module-level `failed` flag and a single DEV `console.info` is written: **an ungraded preview beside a graded export is recoverable; a preview that throws every frame at 60fps is not.** The user still has a way to see the true result, because the paused exact frame is the export's own graded output.
- **The GL context was lost** (see below). Same latch, same one-time note.

**A lost context must never reach the blit.** If the driver drops the context - a Windows TDR driver reset is the realistic trigger, and this process also runs wgpu for exports - the GL canvas goes blank and `drawArrays` silently no-ops, but the `copy` blit at the end of this function would still run and would replace the whole visible preview with that blank canvas, every frame, forever. So `gl.isContextLost()` is checked before the upload and `giveUp` runs instead: the context is released, the latch is set, and the loop falls back to the ungraded picture with one DEV note. `build()` also registers a `webglcontextlost` listener on the GL canvas that calls `preventDefault` and drops `held`, so a loss that is delivered as an event before the next frame releases the dead context on its own; whichever arrives first, nothing blank is ever copied over the preview.

**The context is lazy and held for the life of the page.** Building it means compiling two shaders and linking a program, which is milliseconds, and it must not happen on a frame that is not going to draw. Once built it is reused; the canvas is resized only when the stage canvas size actually changes, and the texture is re-uploaded from the 2D canvas each frame with `texImage2D`, which is the one unavoidable per-frame cost.

**The uniform slots deliberately mirror `fx_uniforms::pack_grade`'s six vec4 one for one**, so a reader comparing the export's packing with the preview's sees the same layout rather than two arrangements of the same eleven numbers.

**`globalCompositeOperation = "copy"` is what makes this a replacement of the frame rather than a draw over it.** A grade is a remap, so the graded pixels must REPLACE the originals; drawing with the default `source-over` would composite the result on top of the ungraded frame and, because the shader preserves source alpha, would mostly look right and be subtly wrong wherever alpha is not 1. The `save`/`restore` pair around it is what keeps that mode from leaking into the cursor and overlay draws that follow.

The texture is `NEAREST` filtered and `CLAMP_TO_EDGE` wrapped: the draw is one to one with the canvas, so there is nothing to interpolate and no edge to wrap.

### Used by

- `src/editor/hooks/stage/compositeFrame.ts` (`drawCompositeFrame`) - the only caller, once per composited frame, in the Task 0a seam

### Why there is no unit test

`gradeCanvas` needs a real WebGL2 context. Vitest's default `node` environment has no `document`, and the `jsdom` environment some suites opt into implements `canvas.getContext("webgl2")` as `null`, so the only behaviour a test could assert here is the `false` path, which asserts nothing about the pixels. Mocking a GL context would pin the sequence of calls this file happens to make rather than the image it produces, which is worse than no test: it would have to be rewritten every time the file is, and would still pass if the shader were wrong.

What guards it instead: `gradeShader.test.ts` pins the shader source's constants and step order, `grade.test.ts` pins the arithmetic those constants feed against 126 rows generated by the Rust reference, and the paused exact frame (`useExactFrame`) is the export's own graded output, which has been the standing visual gate since 2026-09-14.

## giveUp

```ts
function giveUp(why: string): false
```

Release the context, latch `failed`, write the one DEV note and return `false`. Not exported; it exists so the two ways this pass can stop (no WebGL2 at all, and a lost context) cannot drift apart in what they leave behind. Returning `false` rather than `void` is what lets both call sites be a single `return giveUp(...)` line.

**`held` and `failed` are module state with no reset, deliberately.** There was an exported `resetGradePassForTests` for a while; it never had a caller and could not get one, because jsdom has no WebGL2 and so no test in this tree ever reaches `build` at all - `gradeCanvas` returns on `paramsOf` or latches `failed` on the first call and every later one is a single compare. The preview's numbers are pinned instead on `gradeCpu.ts`, a transcription that draws nothing and needs no context. If a browser-run test ever wants the latch cleared, it should come back with that test and not before.
