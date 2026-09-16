# src-tauri/src/export/fx/mask/maskdraw.rs

The CPU fallback for the three mask kinds, painting straight onto a BGRA frame. `fx_mask.wgsl` is the reference look, not this file: `select_fx` picks the GPU renderer whenever an adapter exists and the editor preview goes through the same selector, so on any machine with a GPU neither the export nor the preview runs this code. It is the adapter-less path, and the same standing rule `spotdraw` carries applies - where the two can diverge, this file moves toward the shader.

**Two stated approximations**, of exactly the kind `spotdraw` already documents for its Blur and Nebula spotlight modes:

1. **The blur kernel.** The GPU does a fixed 13-tap disc in ONE pass, so its cost is flat in the radius and only the spread grows. Here it is three box passes, which is a real Gaussian approximation and is affordable per frame on the bounded box a mask covers. Both read as "blurred" at any radius a user picks; a smooth gradient can differ in its last bit between them.
2. **Nothing else.** The feather curve, the coverage, the pixel grid's origin and the highlight multiply are line-for-line mirrors of the shader.

## blur_sigma

```rust
pub fn blur_sigma(amount_px: f32) -> f32
```

The Gaussian sigma that three box passes of radius `round(amount_px)` approximate: `sqrt(((2r + 1)^2 - 1) / 4)`, with `r` floored at one pixel so a zero-strength mask still has a defined kernel rather than dividing by nothing.

*Why it is public and why it exists at all:* the canvas preview cannot run box passes cheaply, so it blurs with `ctx.filter = blur(Npx)`, which is a CSS **Gaussian** and takes a SIGMA, not a radius. Handing it this number instead of `amount_px` is what makes a 24 px export blur and a 34.6 px CSS blur the same picture. `maskPreview.ts::blurSigmaFor` is the TypeScript twin and `blur_sigma_is_the_three_box_approximation` pins both to the same three sample points.

### Used by

- `src/editor/stage/mask/maskDraw.ts` (`blur`) - through its `blurSigmaFor` mirror, as the CSS filter's argument.

## draw_masks

```rust
pub fn draw_masks(out: &mut [u8], ow: u32, oh: u32, masks: &[MaskDraw])
```

Paints every mask in `masks` onto the BGRA frame `out`, in the order given (which `masks_at` has already put in layer order). Returns immediately on an empty list, and skips any mask at alpha 0 - `alpha_zero_is_a_byte_identical_no_op_for_every_kind` pins that a faded-out mask leaves the frame byte for byte untouched, not merely close.

### The one ordering rule

**The frame is snapshotted ONCE, before anything is drawn, and blur and pixelate sample that snapshot rather than the running buffer.** On the GPU, `textureSample` always reads the uploaded frame, so a second mask overlapping a first sees the ORIGINAL pixels there; without the snapshot the CPU would blur an already-blurred region and the two paths would diverge visibly wherever two masks touch. `two_masks_read_the_same_source_frame` is the guard: a pixel under two overlapping blurs must land within 2 of where one blur alone put it.

Highlight is the exception that needs no snapshot, because it multiplies in place and never samples.

### The three kinds

- **`kind` 1, blur.** Copies the mask's padded bounding box into a float scratch buffer, runs three horizontal-then-vertical box passes of radius `round(amount_px)` over it (a running sum, so cost is independent of the radius), and mixes the result back at the per-pixel coverage. Only the bounding box is touched, which is what `blur_leaves_the_outside_of_the_rect_untouched` checks byte for byte.
- **`kind` 2, pixelate.** For each covered pixel, snaps to the cell grid and reads the snapshot at the cell's CENTRE. *Why the grid is anchored to `mn` rather than to the frame origin:* a grid fixed to the frame makes the cells crawl under the rectangle while the user drags it, because the rect slides across a stationary lattice. Anchored to the rect, the cells move with it.
- **`kind` 3, highlight.** Multiplies every pixel by `1 - dim * alpha * (1 - coverage)`, so the inside keeps its brightness and everything outside darkens. *Why it iterates the WHOLE frame* where the other two stay in the bounding box: the pixels it changes are precisely the ones outside the rect.

Coverage throughout is `clamp(0.5 - rrect_sd(px + 0.5) / max(feather_px, 1), 0, 1) * alpha`: the half-pixel convention of `rrect_cov`, widened by the feather. Sampling at the pixel CENTRE (`+ 0.5`) is what keeps the edge from sitting half a pixel off the shader's.

### Used by

- `src-tauri/src/export/fx/fxdraw.rs` (`CpuFx::apply`) - the first step of the CPU effect stack, before the grade, the video FX, the spotlight and the clicks.
- `src-tauri/src/export/fx/fx_state.rs` (`render`) - directly, for any mask beyond the eighth, which the GPU uniform block has no slot for.
