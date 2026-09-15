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
  const w = c.width,
    h = c.height;
  const rect = c.getBoundingClientRect();
  const canvasPxX = ((clientX - rect.left) / rect.width) * w;
  const canvasPxY = ((clientY - rect.top) / rect.height) * h;

  const x = w > 0 ? canvasPxX / w : 0.5;
  const y = h > 0 ? canvasPxY / h : 0.5;

  return [Math.max(0, Math.min(1, x)), Math.max(0, Math.min(1, y))];
}
