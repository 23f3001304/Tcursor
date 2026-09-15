import type { CursorKindSample } from "../../../shared/ipc";

export const GLASS_ALPHA = 0.65;

export const LENS_ZOOM = 1.35;

export const BACK_SCALE = 2.2;

export const PILL_W = 0.35;

export const MORPH_MS = 160;

export function morphEase(p: number): number {
  const q = 1 - Math.min(1, Math.max(0, p));
  return 1 - q * q * q;
}

export function cursorMorphAt(
  kinds: CursorKindSample[],
  ms: number,
): { kind: string; prev: string; m: number } {
  let lo = 0,
    hi = kinds.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (kinds[mid].t <= ms) lo = mid + 1;
    else hi = mid;
  }
  if (lo === 0) return { kind: "arrow", prev: "arrow", m: 1 };
  const at = kinds[lo - 1];
  return {
    kind: at.kind,
    prev: lo >= 2 ? kinds[lo - 2].kind : "arrow",
    m: morphEase((ms - at.t) / MORPH_MS),
  };
}

export function spriteBox(
  img: HTMLImageElement,
  hot: [number, number],
  canvasH: number,
  p: [number, number],
  sizePx: number,
): [number, number, number, number] {
  const scale = sizePx / Math.max(canvasH, 1);
  const w = img.naturalWidth * scale,
    h = img.naturalHeight * scale;
  return [p[0] - hot[0] * w, p[1] - hot[1] * h, w, h];
}

export function lerpBox(
  a: [number, number, number, number],
  b: [number, number, number, number],
  m: number,
): [number, number, number, number] {
  const t = Math.min(1, Math.max(0, m));
  return [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
    a[3] + (b[3] - a[3]) * t,
  ];
}

export function backBox(kind: string, spriteH: number): { rx: number; ry: number } {
  const r = (spriteH * BACK_SCALE) / 2;
  return kind === "ibeam" ? { rx: r, ry: r * PILL_W } : { rx: r, ry: r };
}

export function drawBack(
  ctx: CanvasRenderingContext2D,
  centre: [number, number],
  kind: string,
  prev: string,
  m: number,
  spriteH: number,
) {
  const a = backBox(prev, spriteH),
    b = backBox(kind, spriteH);
  const t = Math.min(1, Math.max(0, m));
  const rx = a.rx + (b.rx - a.rx) * t,
    ry = a.ry + (b.ry - a.ry) * t;
  if (rx < 0.5 || ry < 0.5) return;
  ctx.save();
  ctx.beginPath();
  ctx.roundRect(centre[0] - rx, centre[1] - ry, rx * 2, ry * 2, Math.min(rx, ry));
  ctx.clip();
  const sw = (rx * 2) / LENS_ZOOM,
    sh = (ry * 2) / LENS_ZOOM;
  ctx.drawImage(
    ctx.canvas,
    centre[0] - sw / 2,
    centre[1] - sh / 2,
    sw,
    sh,
    centre[0] - rx,
    centre[1] - ry,
    rx * 2,
    ry * 2,
  );
  ctx.fillStyle = "rgba(235, 243, 255, 0.10)";
  ctx.fill();
  ctx.strokeStyle = "rgba(255, 255, 255, 0.30)";
  ctx.lineWidth = 1;
  ctx.stroke();
  ctx.restore();
}
