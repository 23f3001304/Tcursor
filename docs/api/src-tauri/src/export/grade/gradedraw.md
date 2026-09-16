# src-tauri/src/export/grade/gradedraw.rs

The colour grade's CPU path: one per-pixel loop over the composited BGRA frame, calling `export::grade::apply_px`. It is step 4 of the composite order (spec 1.3) on the software path, reached from `fx::fxdraw::CpuFx::apply` after the masks and before the video FX. The GPU path renders the same eight steps in `export/fx/fx_grade.wgsl` instead; the two are never both run on a frame.

## draw_grade

```rust
pub fn draw_grade(out: &mut [u8], ow: u32, oh: u32, p: &GradeParams)
```

Grades `out` in place. `out` is the composited frame as **BGRA** bytes, four per pixel, row major and tightly packed at `ow` pixels per row: byte `i` is blue, `i + 1` green, `i + 2` red, `i + 3` alpha. `apply_px` takes and returns RGB, so the read swaps `out[i + 2]` into channel 0 and the write swaps it back. Returns immediately on a zero-sized frame rather than indexing an empty slice.

**The alpha byte is never read and never written.** The composited frame is opaque by the time the FX pass sees it, but the overlay and preview paths both rely on alpha surviving the pass untouched, and a grade is a colour operation with no business changing coverage.

**The pixel-centre convention is load bearing.** `u` is `(x + 0.5) / ow - 0.5` and `v` is `(y + 0.5) / oh - 0.5`, so both run from just inside -0.5 to just inside +0.5 and the centre of an odd-sized frame lands on exactly 0. The vignette is the only step that reads them, and getting the half-pixel wrong shifts its centre by half a pixel against the GPU path, which the parity table would catch as a one-byte drift at the frame edges.

**`apply_px` is called per pixel rather than hoisting its invariants.** `exp2(exposure)`, the reciprocal gammas and the corner normaliser could all be lifted out of the loop, and spec 3.4 originally said to. They are not, because the whole function is eleven multiplies and one `powf` per channel, the FX pass on this path already walks the frame several times, and a hoisted variant would be a fifth transcription of the eight steps to keep in step with the other four. The cost of a divergence there is a preview that silently disagrees with the export; the cost of not hoisting is unmeasurable beside the decode.

### Used by

- `src-tauri/src/export/fx/fxdraw.rs` (`CpuFx::apply`) - the only call site, gated on `FxState.grade` being `Some`

### Behaviors

- `the_loop_agrees_with_apply_px_at_every_pixel` - a 17x13 gradient frame under Midnight, every pixel and every channel compared against `apply_px` called directly with the same `u` and `v`. Deliberately an odd size on both axes, so a row-stride or half-pixel error cannot hide.
- `the_alpha_channel_is_never_touched` - every alpha byte set to 123 survives a Vivid grade.
- `noir_leaves_three_equal_channels_everywhere` - Noir's saturation of zero produces three equal channels at every pixel of the frame, which is the cheapest end-to-end check that the loop really runs the saturation step and not just the multiplies.
