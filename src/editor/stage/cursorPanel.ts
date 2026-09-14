// The export's synthetic cursor scales AND clips to the screen panel (cursorset.rs: "scaled to
// the panel size ... and clipped to the panel's on-screen rect so it never spills onto the
// background or the webcam"). Pulled out of previewCanvas.ts as pure ratio/projection math (no
// canvas, no DOM) so this exact mirror is unit-tested directly rather than eyeballed on screen.

/** The cursor's scale-down factor for a screen panel of width `screenW`, against the export's
 *  fixed reference inset width `insetW` (`LayoutPresets.inset_w` - a baseline independent of the
 *  active preset/arrangement, NOT the panel's own preset rect). Both args must be the same unit
 *  (canvas px, or a fraction of the canvas - it's a pure ratio, so either works); `previewCanvas.ts`
 *  passes fractions since that's what `PreviewLayout.screen`/`LayoutPresets.inset_w` already are.
 *  Mirrors `cursorset.rs`'s `(screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0)` exactly, except
 *  the divide-by-zero floor is a tiny epsilon rather than "1 output px" (which has no fixed meaning
 *  in fraction units) - it only ever matters for a degenerate zero `insetW`, never a real one. */
export function panelFactor(screenW: number, insetW: number): number {
  const f = screenW / Math.max(insetW, 1e-6);
  return Math.min(1, Math.max(0.1, f));
}

/** Canvas px per SOURCE px for the screen panel - the scale that keeps the CAPTURED cursor at its
 *  true size relative to the screen content. The recorded bitmaps are in source pixels, and the
 *  source is drawn into the screen panel, so one source pixel is `panelWidth / srcW` canvas px;
 *  written as `panelFactor` times the inset's own source->canvas ratio, the same number with the
 *  panel shrink applied exactly ONCE. Mirrors Rust `export::cursor::captured::content_scale`.
 *
 *  `insetPx` is the reference inset width in CANVAS px (`LayoutPresets.inset_w * canvasW` - the
 *  fraction scaled up, since `srcW` is in real pixels and the ratio has to share a unit).
 *  A non-positive `srcW` means the backend could not probe the video: falls back to `panel`
 *  alone, i.e. the pre-content-scale behavior, rather than collapsing the cursor to nothing.
 *
 *  `srcW` is the source width the panel actually SHOWS - the whole recorded frame normally, but
 *  only the active source span's slice of it after a mid-take display switch (the caller scales it
 *  by `PreviewLayout.src`'s width). Rust does the same, passing `Scene.src.w` where it used to pass
 *  the canvas width; the full width after a switch would size the cursor against pixels the panel
 *  is not drawing. */
export function contentScale(panel: number, insetPx: number, srcW: number): number {
  return srcW > 0 ? panel * insetPx / srcW : panel;
}

/** The zoom crop `previewCanvas.ts` resizes the whole base frame through (its `cx0/cy0/cw/ch`) -
 *  same shape as the export's `coordmap::crop`. */
export interface ZoomCrop { cx0: number; cy0: number; cw: number; ch: number }

/** Project `rect` (pre-zoom canvas px, the offscreen base-frame space `drawPreview` composites
 *  into - e.g. the screen panel's own `dx/dy/dw/dh`) through `crop` the same way a point is mapped
 *  to its on-screen position, then clamp to the canvas bounds. Mirrors `cursorset.rs`'s
 *  `project_rect` (in turn `coordmap::project` on each corner + `coordmap::crop`) - the box the
 *  export confines the cursor AND its motion trail to, so neither ever spills onto the background
 *  or the webcam. Returns post-zoom canvas px as `[x0, y0, x1, y1]`. Deliberately NOT normalized
 *  when the panel falls entirely outside the crop (`x0` can end up past `x1`) - exactly like the
 *  Rust tuple, which relies on its caller's `ox_start >= ox_end` no-op check rather than
 *  reordering; `drawCursorSprite` carries the same guard before it draws anything. */
export function panelClipRect(
  rect: { x: number; y: number; w: number; h: number },
  crop: ZoomCrop,
  canvasW: number, canvasH: number,
): [number, number, number, number] {
  const project = (px: number, py: number): [number, number] =>
    [(px - crop.cx0) * canvasW / crop.cw, (py - crop.cy0) * canvasH / crop.ch];
  const [x0, y0] = project(rect.x, rect.y);
  const [x1, y1] = project(rect.x + rect.w, rect.y + rect.h);
  return [Math.max(0, x0), Math.max(0, y0), Math.min(canvasW, x1), Math.min(canvasH, y1)];
}
