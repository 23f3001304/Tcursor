export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function alphaBounds(data: Uint8ClampedArray, w: number, h: number, threshold = 8): Box | null {
  let minX = w,
    minY = h,
    maxX = -1,
    maxY = -1;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (data[(y * w + x) * 4 + 3] < threshold) continue;
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      if (y < minY) minY = y;
      if (y > maxY) maxY = y;
    }
  }
  if (maxX < 0) return null;
  return { x: minX, y: minY, w: maxX - minX + 1, h: maxY - minY + 1 };
}

export function fitBox(
  box: Box,
  imgW: number,
  imgH: number,
  fitW: number,
  fitH: number,
  plateW: number,
  plateH: number,
): { width: number; height: number; left: number; top: number } {
  const scale = Math.min(fitW / box.w, fitH / box.h);
  return {
    width: imgW * scale,
    height: imgH * scale,
    left: plateW / 2 - (box.x + box.w / 2) * scale,
    top: plateH / 2 - (box.y + box.h / 2) * scale,
  };
}
