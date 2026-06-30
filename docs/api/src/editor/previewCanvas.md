# src/editor/previewCanvas.ts

The editor preview's Canvas2D compositor: given the screen `<video>`, the webcam `<video>`, the interpolated camera pose, and the export framing/background, it draws one full preview frame - background, the rounded zoomed screen with cursor + click ripples + spotlight, and the webcam PiP - so the preview approximates the export. Native `drawImage` keeps it at 60fps. The zoom magnifies the screen *within* its fixed panel (a source-rect crop), which is why framing differs from the export at high zoom (a documented approximation).

## DrawCam

```ts
export interface DrawCam { scale: number; cx: number; cy: number; curx: number; cury: number }
```

The camera pose for one frame, interpolated from the backend `CamSample` track by `camAt`: the zoom `scale`, the zoom centre `cx`/`cy`, and the cursor `curx`/`cury` - all 0..1 fractions of the screen content. `drawPreview` turns `scale` + centre into a source-rect crop and maps the cursor/clicks through that same crop so they track the zoom.

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

Everything needed to draw the real export cursor: the recording's cursor `style`/`size` + click-bounce + motion-blur settings, the cursor-shape track (`kinds`), the decoded sprite images by lowercase type name (`sprites`) with their hotspots (`hots`, 0..1 of the sprite) and original canvas heights (`canvasH`) for uniform scaling, and `recent` - a small ring of recent on-canvas positions used to draw the motion trail. The cursor is drawn only for `style === "enhanced"` (System = the OS cursor is already baked into the capture; Hidden = none).

## drawPreview

```ts
export function drawPreview(
  ctx: CanvasRenderingContext2D, w: number, h: number,
  screen: HTMLVideoElement, webcam: HTMLVideoElement | null, cam: DrawCam,
  layout: PreviewLayout | null, bg: HTMLImageElement | null, clicks: ClickSample[], now: number,
  cursor: DrawCursor | null, spotlight: SpotlightInput | null,
): void
```

Composites one preview frame onto `ctx` (a `w`x`h` canvas).

### Inputs (what, and why it is needed)

- `ctx`, `w`, `h` - the 2D context and canvas size. *Why:* the backing store is fixed (1280x720) and CSS scales it.
- `screen` - the screen proxy `<video>`. *Why:* its frame is the zoomed source; `videoWidth/Height` give the crop basis.
- `webcam` - the webcam `<video>` or `null`. *Why:* drawn as the PiP panel (cover-fit, rounded) when present and non-empty.
- `cam: DrawCam` - the interpolated camera pose. *Why:* defines the source-rect crop (`sw = vw/scale`, centred on `cx`/`cy`, clamped into the frame) and the cursor position.
- `layout: PreviewLayout | null` - the export framing (screen rect + radius + webcam rect) as fractions. *Why:* so the preview frames the screen/webcam exactly like the export; `null` uses an inset fallback.
- `bg: HTMLImageElement | null` - the decoded export background. *Why:* painted under the screen once loaded, else a gradient placeholder.
- `clicks: ClickSample[]`, `now: number` - the click track and current output time (ms). *Why:* expanding ripples are drawn for clicks within ~500ms of `now`, mapped through the crop.
- `cursor: DrawCursor | null` - the cursor inputs, or `null` to skip. *Why:* drawn (Enhanced only) at the mapped cursor position with bounce + motion trail.
- `spotlight: SpotlightInput | null` - the editable regions + recorded holds + toggle + params, or `null` when click-FX is disabled. *Why:* `spotlightAlpha` computes the strength and `drawSpotlight` darkens the canvas with a soft hole at the cursor (the unclamped cursor canvas position is the spotlight centre).

### Returns

`void` - draws directly onto `ctx`.

### Implementation

1. Paint the background (exact image, else gradient).
2. Resolve the screen rect from `layout` (or an inset fallback) and, if the video has dimensions, derive the source-rect crop from `cam` and `drawImage` the zoomed screen into a clipped rounded rect with a drop shadow.
3. Define a `map(fx, fy)` that projects a 0..1 screen-content point through the crop to a canvas pixel (or `null` when off-crop); draw click ripples through it *inside* the screen clip; record the mapped cursor position (`cpos`) and the unclamped cursor canvas position (`curC`, the spotlight centre). Then `restore()` and draw the cursor sprite **on top, unclipped** - so it is never cut at the panel edge or rounded corner when zoomed in (the export's panel-rect cursor clip expands to ~the whole frame at zoom, so it isn't cut there either). The one divergence: un-zoomed (scale≈1) the export still clips the cursor at the panel rect, whereas the preview now lets the sprite overhang a few px onto the bezel near a screen edge - an accepted trade vs the much more visible zoomed-cut. The webcam PiP draws afterwards, covering the cursor where they overlap, as in the export.
4. Draw the webcam PiP (rounded rect from `layout.cam`, else a bottom-right circle).
5. If a spotlight is active, darken the whole canvas with a soft hole at the cursor.

### Notes

- Private helpers in this file: `coverDraw` (centre-crop cover fit), `drawClicks` (ripple rings), `cursorAt` (binary-search the active cursor kind), `drawCursorSprite` (sprite + motion trail + click bounce), and `roundRect` (rounded-rect path). The spotlight itself lives in `spotlightPreview.ts`.
