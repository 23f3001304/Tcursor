# src-tauri/src/export/cursor/draw/mod.rs

The DRAWING half of `export/cursor`: given a point and a cursor type, put the pixels on the frame. The other half - what a cursor pack is and where its sprites come from - is `cursor/pack.rs` and the `cursor/pack/` folder beside it. This file is the module list only; every symbol lives on the pages below.

## busy

Submodule (`cursor/draw/busy.rs`). Pack format v2's animated busy cursor: pure math turning an output-clock timestamp into "which frame, rotated how far, scaled how much". Key items: `BusyAnim` (spin/flip/pulse), `BusySpec` (a pack's declared animation plus its explicit frame count), `BusyPose`, `busy_pose` - full per-symbol docs in `cursor/draw/busy.md`. Mirrored in TS by `src/editor/stage/cursorBusy.ts`.

## captured

Submodule (`cursor/draw/captured.rs`). The REAL OS cursor, composited from the layer the recorder captured (`events::track::cursorlayer`) - what "System" means on any recording made since screen capture went cursor-free. Draws the actual recorded bitmap at the raw recorded point, with none of the Enhanced polish. Key items: `draws_captured` (the single gate that picks captured over synthetic), `CapturedCursors`, `CapturedCursors::load`, `CapturedCursors::sprite_at`, `CapturedCursors::draw` - full per-symbol docs in `cursor/draw/captured.md`.

## cursordraw

Submodule (`cursor/draw/cursordraw.rs`). CPU rasterizer for the Enhanced synthetic cursor: sprite placement, click-bounce scale animation, and motion trail blending, clipped to the screen panel. Key items: `CursorSprite`, `decode_sprite`, `bounce_scale`, `draw_cursor`, `draw_cursor_posed`, `apply_enhanced` - full per-symbol docs in `cursor/draw/cursordraw.md`.

## cursormorph

Submodule (`cursor/draw/cursormorph.rs`). Cross-fading one cursor STATE into the next, for a `material: "glass"` pack only - the arrow does not cut to the I-beam, it dissolves through a moving box between the two poses. Key items: `morph_box`, `MorphState` - full per-symbol docs in `cursor/draw/cursormorph.md`.

## cursorxform

Submodule (`cursor/draw/cursorxform.rs`). The rotated/scaled blit the animated busy cursor needs: destination-driven inverse mapping with premultiplied bilinear sampling, reached only when `BusyPose::is_identity` is false so every still cursor keeps `cursordraw`'s nearest-neighbour fast path. Key item: `blit_transformed` - full per-symbol docs in `cursor/draw/cursorxform.md`.

## tilt

Submodule (`cursor/draw/tilt.rs`). The motion lean: a low-pass on the drawn cursor's velocity feeding a lightly under-damped spring, on a fixed 1 ms substep grid so a 30 fps export matches a 60 fps one. Key items: `REF_W`, `MAX_DEG`, `Tilt`, `Tilt::step`, `target_deg`, `max_deg`, `ref_scale` - full per-symbol docs in `cursor/draw/tilt.md`. Mirrored in TS by `src/editor/stage/cursorTilt.ts`, pinned against the same five instants.
