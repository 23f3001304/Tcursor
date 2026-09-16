# src-tauri/src/export/fx/mask/mod.rs

Submodule overviews for the **mask** group: the rectangular regions a user draws over the picture to blur a password, pixelate a name or highlight one corner of the frame. A mask is an `EffectRegion` whose `kind` is `Blur`, `Pixelate` or `Highlight` (`EffectKind::is_mask`), authored in canvas fractions and projected to output pixels every frame.

The group is a folder rather than three loose files at the top of `export/fx/` for the reason `click`, `spot`, `lens` and `caption` already are: one overlay's data model and its renderers belong together, and `export/fx/` is long past four files.

`export/fx/mod.rs` re-exports the leaf modules (`pub use self::mask::{fx_masks, maskdraw};`) so call sites outside the group keep short paths, the same shape `pub use self::caption::captiondraw;` already has.

The group also carries **`fx_mask.wgsl`**, which is not a Rust module and so has no doc page of its own: `fx_gpu_pipeline::build_pipeline` concatenates it onto `fx.wgsl`, so it sees that file's `u`, `frame_tex` and `samp`. It holds `rrect_sd` (the WGSL transcription of the one below), `mask_cov` and `mask_fx`, and its kind ids mirror `fx_masks::mask_kind_id` exactly.

Two shape helpers are re-exported at this level (`pub use self::rrect::{rrect_cov, rrect_sd};`) so callers outside the group write `export::fx::mask::rrect_sd` rather than naming the leaf file. That exists for `spot::spotdraw`, which is the one caller in a sibling folder, and for Track 2c's text plate after integration.

## fx_masks

The projection: a mask's canvas-fraction rect becomes the output rectangle this frame draws, with its fade, its corner radius and the kind's strength already resolved. Key items: `MaskDraw` (one mask in output pixels), `mask_kind_id(kind) -> u32` (0 empty, 1 Blur, 2 Pixelate, 3 Highlight), `masks_at(effects, scene, cam, src_full, ow, oh, region_t, default_dim) -> Vec<MaskDraw>`, `HIDDEN_PANEL_ALPHA`. See `mask/fx_masks.md`.

## maskdraw

The CPU fallback: a real three-pass box blur over the rect's bounding box, a pixel grid anchored to the rect's own origin, and a multiply outside the rounded rect for highlight, with one snapshot of the frame taken before any mask draws. Key items: `draw_masks(out, ow, oh, masks)`, `blur_sigma(amount_px) -> f32` (the sigma the preview's CSS blur is handed). See `mask/maskdraw.md`.

## rrect

The one rounded-rectangle shape in the tree, as a signed distance and as an antialiased coverage. Key items: `rrect_sd(x, y, mn, mx, r) -> f32` (signed distance in output pixels, negative inside), `rrect_cov(x, y, mn, mx, r) -> f32` (`(0.5 - sd)` clamped, the half-pixel antialiased coverage). See `mask/rrect.md`.
