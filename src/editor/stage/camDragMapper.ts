/** Maps a pointer position over the preview `<canvas>` to a plain 0..1 fraction of the
 *  displayed frame - NOT un-projected through the screen zoom/crop. Unlike `zoomTargetMapper`
 *  (which targets the screen layer, so it must invert the current zoom crop to find the
 *  pre-zoom point), the webcam PiP is the FIXED TOP layer in OUTPUT space (see the Rust
 *  `Scene.camera` - it's composited after the screen crop, never zoomed/cropped itself), so
 *  the PiP center is always just the pointer's fraction of the whole displayed 1280x720 frame.
 *  Reuses only the canvas-display-rect (letterbox) math from `zoomTargetMapper`. */
export function mapPointerToCamFraction({
  clientX,
  clientY,
  canvasElement,
}: {
  clientX: number;
  clientY: number;
  canvasElement: HTMLCanvasElement;
}): [number, number] {
  const c = canvasElement;
  const w = c.width, h = c.height;
  const rect = c.getBoundingClientRect();
  const canvasPxX = ((clientX - rect.left) / rect.width) * w;
  const canvasPxY = ((clientY - rect.top) / rect.height) * h;

  const x = w > 0 ? canvasPxX / w : 0.5;
  const y = h > 0 ? canvasPxY / h : 0.5;

  return [Math.max(0, Math.min(1, x)), Math.max(0, Math.min(1, y))];
}
