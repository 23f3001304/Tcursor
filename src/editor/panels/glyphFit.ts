// Fitting a cursor sprite into a tile. Every pack ships 128x128 PNGs, but the drawing inside them
// is padded differently per pack (content runs roughly 70 to 105px wide), so drawing the files at
// one fixed size makes the same arrow look big in one tile and small in the next. These two pure
// functions measure the drawing's own bounds and place it, and nothing here touches the pixels:
// the pack's colours are what the pack ships (owner ruling, 2026-09-13).

export interface Box { x: number; y: number; w: number; h: number }

/** The tight bounds of everything at least `threshold` opaque in an RGBA buffer, or `null` when
 *  the image is fully transparent (nothing to fit). `w`/`h` are the buffer's own dimensions. */
export function alphaBounds(data: Uint8ClampedArray, w: number, h: number, threshold = 8): Box | null {
  let minX = w, minY = h, maxX = -1, maxY = -1;
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

/** Where to put the WHOLE image so that `box` - its drawn content - fills `fit` as far as it can
 *  without distortion, centred in a `plate`-sized square. Returns CSS pixels for an absolutely
 *  positioned `<img>`; the image is never cropped or scaled per-axis, so a pack that draws a tall
 *  I-beam and one that draws a wide resize arrow come out the same visual weight. */
export function fitBox(box: Box, imgW: number, imgH: number,
  fitW: number, fitH: number, plateW: number, plateH: number): { width: number; height: number; left: number; top: number } {
  const scale = Math.min(fitW / box.w, fitH / box.h);
  return {
    width: imgW * scale,
    height: imgH * scale,
    left: plateW / 2 - (box.x + box.w / 2) * scale,
    top: plateH / 2 - (box.y + box.h / 2) * scale,
  };
}
