# src/editor/stage/cursorPreview.ts

Draws the real export cursor sprite in the editor preview: resolves which cursor-shape sprite is active at a time, then blits it (with click-bounce and a fading motion trail) at a mapped canvas position. Kept separate from `previewCanvas.ts` since it's reused as-is regardless of how the position was projected (raw crop, or the whole-frame zoom crop).

## DrawCursor

```ts
export interface DrawCursor {
  style: string; size: number; clickBounce: boolean; bounceIntensity: number; motionBlur: number;
  kinds: CursorKindSample[];
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  recent: [number, number][];
}
```

Everything needed to draw the real export cursor: the recording's cursor `style`/`size` + click-bounce + motion-blur settings, the cursor-shape track (`kinds`), the decoded sprite images by lowercase type name (`sprites`) with their hotspots (`hots`, 0..1 of the sprite) and original canvas heights (`canvasH`) for uniform scaling, and `recent` - a small ring of recent on-canvas positions used to draw the motion trail. The cursor is drawn only for `style === "enhanced"` (Hidden = none; System = the OS cursor is already baked into the capture). The gate stays a plain style check on purpose: for a `System` recording with NO baked cursor, `Stage` substitutes an effective `"enhanced"` style (plus an empty `kinds` array, no bounce and no trail) so the plain-OS fallback reuses this exact draw path - see `Stage.md`.

### Used by

- `src/editor/stage/previewCanvas.ts` - `drawPreview` passes the zoom-projected cursor position through to `drawCursorSprite`.

## cursorAt

```ts
export function cursorAt(kinds: CursorKindSample[], ms: number): string
```

The cursor type active at output time `ms` (binary search: the last sample with `t <= ms`), default `"arrow"`.

## drawCursorSprite

```ts
export function drawCursorSprite(
  ctx: CanvasRenderingContext2D, p: [number, number], now: number,
  c: DrawCursor, clicks: ClickSample[], outH: number,
  panel: number, clip: [number, number, number, number],
): void
```

Draws the cursor sprite at the already-projected canvas position `p`, scaled by `panel` and clipped to `clip` - mirrors the export's `cursorset::draw`, which scales AND clips the cursor to the screen panel (see `cursorPanel.ts`'s `panelFactor`/`panelClipRect`, the pure functions `previewCanvas.ts` derives both from).

### Inputs

- `ctx` - the 2D context to draw onto.
- `p: [number, number]` - the cursor's canvas pixel position (already projected through whatever zoom crop is active). *Why pre-projected:* this file has no idea how the caller derived the position, so it stays reusable across projection models.
- `now: number` - current output time (ms), used for the cursor-kind lookup and click-bounce timing.
- `c: DrawCursor` - style/size/sprite inputs. Gated by `c.style !== "enhanced"` (no-op otherwise).
- `clicks: ClickSample[]` - the click track; a click within 180ms of `now` shrinks the sprite briefly (`bounce_intensity`-scaled dip) when `c.clickBounce` is set.
- `outH: number` - output canvas height; sprite size is `c.size * outH * 0.033 * panel`, independent of any zoom scale (matches the export, which doesn't scale cursor size with zoom either - `panel` is a LAYOUT factor, not a zoom factor).
- `panel: number` - the screen panel's scale-down factor (`cursorPanel.ts`'s `panelFactor`, 0.1..1.0) - a shrunk custom-arrangement panel shrinks the cursor with it, just like a small PiP screen does in the export.
- `clip: [number, number, number, number]` - post-zoom canvas px `[x0, y0, x1, y1]` the cursor + trail are confined to (the screen panel's own on-screen rect, `cursorPanel.ts`'s `panelClipRect`) - so neither ever spills onto the background or the webcam. Possibly "inverted" (`x0` past `x1`) when the panel is entirely outside the current zoom crop; see Implementation.

### Returns

`void`.

### Implementation

1. No-op (before touching `c.recent`) if `c.style !== "enhanced"`, or if `clip` is empty/inverted (`clip[2] <= clip[0] || clip[3] <= clip[1]`) - mirrors the export's `blit`'s `ox_start >= ox_end` guard.
2. Resolve the active sprite/hotspot/canvas-height for `cursorAt(c.kinds, now)`, falling back to `"arrow"`. No-op if the sprite isn't loaded yet.
3. Compute `sizePx` (scaled by `panel`), apply the click-bounce dip if applicable, derive `scale = sizePx / canvasH`.
4. Maintain `c.recent` (a ring of up to 6 positions), resetting it if the cursor jumped more than `outH * 0.2` in one frame (a scene cut / seek, not real motion).
5. `ctx.save()`/clip to `clip`/restore around the actual drawing: if `c.motionBlur > 0`, blit a fading trail from `c.recent` (oldest = most transparent) before the final blit at `p`.
