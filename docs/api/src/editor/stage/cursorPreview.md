# src/editor/stage/cursorPreview.ts

Draws the export cursor in the editor preview. Two cursors live here: the REAL recorded OS cursor, composited from the project's captured layer for the `"system"` style (the mirror of Rust `export::cursor::captured`), and the synthetic Enhanced sprite - resolved by shape at a time, then blitted with click-bounce and a fading motion trail. Kept separate from `previewCanvas.ts` since it's reused as-is regardless of how the position was projected (raw crop, or the whole-frame zoom crop).

## DrawCursor

```ts
export interface DrawCursor {
  style: string; size: number; clickBounce: boolean; bounceIntensity: number; motionBlur: number;
  kinds: CursorKindSample[];
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  captured: CapturedLayer | null;
  busy: BusySpec | null;
  busyFrames: HTMLImageElement[];
  tiltDeg: number;
  recent: [number, number][];
}
```

Everything needed to draw the export cursor: the recording's cursor `style`/`size` + click-bounce + motion-blur settings, the cursor-shape track (`kinds`), the decoded sprite images by lowercase type name (`sprites`) with their hotspots (`hots`, 0..1 of the sprite) and original canvas heights (`canvasH`) for uniform scaling, `captured` (the real recorded cursor layer, or `null`), `busy`/`busyFrames` (pack format v2's animated busy state - the selected pack's declared animation and its explicit frames, empty unless it ships them), and `recent` - a small ring of recent on-canvas positions used to draw the motion trail.

**Which cursor draws.** A non-null `captured` wins and the synthetic path is skipped entirely; `Stage` populates it only when the doc style is `"system"` and the recording has a layer, so this field IS the gate (mirroring Rust `captured::draws_captured`). Otherwise the synthetic sprite is drawn only for `style === "enhanced"` (Hidden = none; System = the OS cursor is already baked into a pre-layer capture). That second gate stays a plain style check on purpose: for a PRE-LAYER `System` recording with no baked cursor and no layer, `Stage` substitutes an effective `"enhanced"` style (plus an empty `kinds` array, no bounce and no trail) so the plain-OS fallback reuses this exact draw path - see `Stage.md`.

### Used by

- `src/editor/stage/previewCanvas.ts` - `drawPreview` passes the zoom-projected cursor position through to `drawCursorSprite`.
- `src/editor/hooks/useCursorSprites.ts` - builds `CapturedLayer` from the `cursor_layer` DTO.

### DrawCursor::material

```ts
material: string | null;
back: string;
```

The two halves of the **glass cursor material**, threaded in by `useCompositeLoop` from the pack DTO and the doc's cursor settings respectively.

- `material` - `"glass"` when the pack's sprites are LENSES (Rust `fx_lens`). The sprite is then drawn at `cursorGlass.ts::GLASS_ALPHA`, and its states CROSS-FADE through one interpolated box instead of snapping.
- `back` - `CursorSettings.back`. `"glass"` draws the disc/pill behind the cursor.

Both live on `DrawCursor` rather than being read from a hook inside the draw, for the same reason every other field does: the draw is a pure function of what it is handed, so `previewCanvas.ts` can call it for a live frame and a test can call it for a fixed one.

The geometry and the numbers are in `cursorGlass.ts`, which also documents everything the canvas deliberately does NOT do (no refraction, no frost, no shadow, no click squash, no selection stretch).

### DrawCursor::back

See `DrawCursor::material`.

### DrawCursor::tiltDeg

```ts
tiltDeg: number;
```

This frame's **motion lean** in degrees (`cursorTilt.ts`), the mirror of what Rust hands `cursorset::draw` as `tilt_deg`. Composed with whatever rotation the busy pose already carries, so a spinning busy cursor thrown across the screen does both at once about the same hotspot - one transform, not two.

*Why it arrives as a number rather than being computed here:* the lean is a filter over time, advanced exactly once per tick, and the draw must stay a pure function of what it is handed. `useCompositeLoop` owns the filter state (next to the motion trail's, and reset on the same discontinuities) and calls `cursorTilt.ts`'s `tiltFromCam` once per frame.

## cursorAt

```ts
export function cursorAt(kinds: CursorKindSample[], ms: number): string
```

The cursor type active at output time `ms` (binary search: the last sample with `t <= ms`), default `"arrow"`.

## CapturedLayer

```ts
export interface CapturedLayer {
  track: [number, number][];
  images: Map<number, HTMLImageElement>;
  hots: Map<number, [number, number]>;
  srcW: number;
}
```

The recording's captured OS-cursor layer, decoded: the `[t, id]` timeline in output ms, each real cursor bitmap by layer id, each one's hotspot in CAPTURED pixels (not a 0..1 fraction, unlike `DrawCursor.hots` - the bitmap is drawn at its own size scaled by `capturedScale`, so pixels are the natural unit), and `srcW`, the recorded video's own width. Built by `useCursorSprites` from the `cursor_layer` command's DTO; mirrors Rust `export::cursor::captured::CapturedCursors`.

*Why `srcW` is on the layer and not read off the `<video>`:* the preview plays a downscaled proxy, so `videoWidth` is not the source width. The bitmaps are in source pixels, so without the real one they cannot be sized against the screen content (`cursorPanel.ts`'s `contentScale`).

## idAt

```ts
export function idAt(track: [number, number][], ms: number): number | null
```

The captured cursor id showing at output time `ms` (binary search: the last sample with `t <= ms`), or `null` before the first sample and on an empty track. The TS mirror of Rust `CursorLayer::id_at`.

*Why `null` rather than a default the way `cursorAt` returns `"arrow"`:* there is no universal fallback BITMAP the way Arrow is a universal fallback shape, so "nothing captured yet" has to mean "draw nothing". Note that `0` is a real id - callers must test `=== null`, not falsiness.

## drawCaptured

```ts
function drawCaptured(ctx: CanvasRenderingContext2D, p: [number, number], now: number,
  cap: CapturedLayer, scale: number, clip: [number, number, number, number]): void
```

Draw the REAL recorded cursor bitmap: hotspot on the already-projected point `p`, at `scale` (canvas px per source px - `cursorPanel.ts`'s `contentScale`), clipped to `clip`. The mirror of Rust `CapturedCursors::draw` - top-left at `p - hot * scale`, size `natural * scale` - so the preview and the export place AND size it identically. No bounce, no trail, no glide: `"system"` is the cursor that was actually on screen, not an idealized one. No-op while the image is still decoding, or before the first track sample.

## drawCursorSprite

```ts
export function drawCursorSprite(
  ctx: CanvasRenderingContext2D, p: [number, number], now: number,
  c: DrawCursor, clicks: ClickSample[], outH: number,
  panel: number, clip: [number, number, number, number], capturedScale: number,
): void
```

Draws the cursor sprite at the already-projected canvas position `p`, scaled by `panel` and clipped to `clip` - mirrors the export's `cursorset::draw`, which scales AND clips the cursor to the screen panel (see `cursorPanel.ts`'s `panelFactor`/`panelClipRect`, the pure functions `previewCanvas.ts` derives both from).

### Inputs

- `ctx` - the 2D context to draw onto.
- `p: [number, number]` - the cursor's canvas pixel position (already projected through whatever zoom crop is active). *Why pre-projected:* this file has no idea how the caller derived the position, so it stays reusable across projection models.
- `now: number` - current output time (ms), used for the cursor-kind lookup and click-bounce timing.
- `c: DrawCursor` - style/size/sprite inputs, and `c.captured`. A non-null `c.captured` takes the captured-bitmap path and returns; otherwise gated by `c.style !== "enhanced"` (no-op).
- `clicks: ClickSample[]` - the click track; a click within 180ms of `now` shrinks the sprite briefly (`bounce_intensity`-scaled dip) when `c.clickBounce` is set.
- `outH: number` - output canvas height; sprite size is `c.size * outH * 0.033 * panel`, independent of any zoom scale (matches the export, which doesn't scale cursor size with zoom either - `panel` is a LAYOUT factor, not a zoom factor).
- `panel: number` - the screen panel's scale-down factor (`cursorPanel.ts`'s `panelFactor`, 0.1..1.0) - a shrunk custom-arrangement panel shrinks the cursor with it, just like a small PiP screen does in the export.
- `capturedScale: number` - canvas px per SOURCE px (`cursorPanel.ts`'s `contentScale`), used by the captured path ONLY. *Why it is not `panel`:* the recorded bitmaps are in source pixels, while the synthetic sprites are authored against the output canvas - sizing the former by `panel` alone drew a 4K take's cursor at roughly twice its on-screen proportion.
- `clip: [number, number, number, number]` - post-zoom canvas px `[x0, y0, x1, y1]` the cursor + trail are confined to (the screen panel's own on-screen rect, `cursorPanel.ts`'s `panelClipRect`) - so neither ever spills onto the background or the webcam. Possibly "inverted" (`x0` past `x1`) when the panel is entirely outside the current zoom crop; see Implementation.

### Returns

`void`.

### Implementation

1. No-op (before touching `c.recent`) if `clip` is empty/inverted (`clip[2] <= clip[0] || clip[3] <= clip[1]`) - mirrors the export's `blit`'s `ox_start >= ox_end` guard.
2. If `c.captured` is non-null, hand off to `drawCaptured` (at `capturedScale`, not `panel`) and return. The trail ring is deliberately left untouched on this path - the captured cursor has no trail, and leaving `c.recent` alone means switching back to Enhanced does not inherit a stale one.
3. No-op if `c.style !== "enhanced"`.
4. Resolve the active sprite/hotspot/canvas-height for `cursorAt(c.kinds, now)`, falling back to `"arrow"`. No-op if the sprite isn't loaded yet.
5. Compute `sizePx` (scaled by `panel`), apply the click-bounce dip if applicable, derive `scale = sizePx / canvasH`.
6. Maintain `c.recent` (a ring of up to 6 positions), resetting it if the cursor jumped more than `outH * 0.2` in one frame (a scene cut / seek, not real motion).
7. `ctx.save()`/clip to `clip`/restore around the actual drawing: if `c.motionBlur > 0`, blit a fading trail from `c.recent` (oldest = most transparent) before the final blit at `p`.

### The busy state animates, and the cursor leans

When the active kind is `"busy"` and the pack declares an animation, `busyPose(c.busy, now)` gives this frame's spin - `now` is the OUTPUT clock, the same basis the export passes `busy_pose`, so a paused preview shows exactly the frame the export would write. A pack shipping explicit `busy_NN.png` frames uses `c.busyFrames[spin.frame]` as the image and carries no synthesised rotation (`busyPose` returns the identity for it).

`c.tiltDeg` is then **added** to that spin's angle, giving the one pose everything is drawn under: for an ordinary upright cursor the pose is still the identity and the draw is the plain `drawImage` it always was; a lean alone, a spin alone, or both together all go through `drawPosed` once. The glass cross-fade's outgoing sprite is posed too, matching Rust `cursormorph::draw_glass`, so a state change mid-throw does not leave one of the two shapes standing upright.

The motion trail is never posed: like the export, it blits the untransformed sprite, because spinning each fading ghost independently reads as noise rather than motion.

## drawPosed

```ts
function drawPosed(ctx: CanvasRenderingContext2D, q: [number, number], pose: BusyPose,
  blit: (q: [number, number]) => void): void
```

Run `blit` under the pose's rotation/scale about the cursor point `q` - the canvas mirror of Rust `cursorxform::blit_transformed`, which rotates about the sprite's hotspot and leaves it on the anchor. Same anchor as the untransformed draw, so a spinning or leaning cursor never drifts off the point it is pointing at.

Geometrically identical to the export, not pixel-identical: this leans on Canvas2D's own resampling rather than reimplementing the bilinear loop, which is the existing arrangement for every other part of the preview.

