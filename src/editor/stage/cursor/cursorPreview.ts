import type { CursorKindSample, ClickSample } from "../../../shared/ipc";
import { busyPose, isIdentity, STILL, type BusyPose, type BusySpec } from "./cursorBusy";
import { cursorMorphAt, drawBack, lerpBox, spriteBox, GLASS_ALPHA } from "./cursorGlass";

export interface CapturedLayer {
  track: [number, number][];
  images: Map<number, HTMLImageElement>;
  hots: Map<number, [number, number]>;
  srcW: number;
}

export interface DrawCursor {
  style: string;
  size: number;
  clickBounce: boolean;
  bounceIntensity: number;
  motionBlur: number;
  kinds: CursorKindSample[];
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  captured: CapturedLayer | null;
  busy: BusySpec | null;
  busyFrames: HTMLImageElement[];
  material: string | null;
  tiltDeg: number;
  back: string;
  recent: [number, number][];
}

export function idAt(track: [number, number][], ms: number): number | null {
  let lo = 0,
    hi = track.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (track[mid][0] <= ms) lo = mid + 1;
    else hi = mid;
  }
  return lo > 0 ? track[lo - 1][1] : null;
}

function drawCaptured(
  ctx: CanvasRenderingContext2D,
  p: [number, number],
  now: number,
  cap: CapturedLayer,
  scale: number,
  clip: [number, number, number, number],
) {
  const id = idAt(cap.track, now);
  if (id === null) return;
  const img = cap.images.get(id);
  if (!img || !img.complete || !img.naturalWidth) return;
  const hot = cap.hots.get(id) ?? [0, 0];
  ctx.save();
  ctx.beginPath();
  ctx.rect(clip[0], clip[1], clip[2] - clip[0], clip[3] - clip[1]);
  ctx.clip();
  ctx.drawImage(
    img,
    p[0] - hot[0] * scale,
    p[1] - hot[1] * scale,
    img.naturalWidth * scale,
    img.naturalHeight * scale,
  );
  ctx.restore();
}

export function cursorAt(kinds: CursorKindSample[], ms: number): string {
  let lo = 0,
    hi = kinds.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (kinds[mid].t <= ms) lo = mid + 1;
    else hi = mid;
  }
  return lo > 0 ? kinds[lo - 1].kind : "arrow";
}

export function drawCursorSprite(
  ctx: CanvasRenderingContext2D,
  p: [number, number],
  now: number,
  c: DrawCursor,
  clicks: ClickSample[],
  outH: number,
  panel: number,
  clip: [number, number, number, number],
  capturedScale: number,
) {
  if (clip[2] <= clip[0] || clip[3] <= clip[1]) return;
  if (c.captured) {
    drawCaptured(ctx, p, now, c.captured, capturedScale, clip);
    return;
  }
  if (c.style !== "enhanced") return;
  const glass = c.material === "glass";
  const morph = cursorMorphAt(c.kinds, now);
  const kind = morph.kind;
  const m = glass ? morph.m : 1;
  const spin = kind === "busy" && c.busy ? busyPose(c.busy, now) : STILL;
  const pose = c.tiltDeg ? { ...spin, angleDeg: spin.angleDeg + c.tiltDeg } : spin;
  const frame = c.busyFrames[spin.frame];
  const img = frame ?? c.sprites.get(kind) ?? c.sprites.get("arrow");
  const ch = c.canvasH.get(kind) ?? c.canvasH.get("arrow");
  const hot = c.hots.get(kind) ?? c.hots.get("arrow");
  if (!img || !img.complete || !img.naturalWidth || !ch || !hot) return;
  const clampedSize = Math.min(Math.max(c.size, 0.4), 3.0);
  let sizePx = clampedSize * outH * 0.033 * panel;
  if (c.clickBounce) {
    for (let i = clicks.length - 1; i >= 0; i--) {
      const dt = now - clicks[i].t;
      if (dt < 0) continue;
      if (dt < 180) sizePx *= 1 - 0.36 * c.bounceIntensity * (1 - dt / 180);
      break;
    }
  }
  const curBox = spriteBox(img, hot, ch, p, sizePx);
  const prevImg = m < 1 ? c.sprites.get(morph.prev) : undefined;
  const prevHot = c.hots.get(morph.prev),
    prevCh = c.canvasH.get(morph.prev);
  const box =
    prevImg && prevImg.complete && prevImg.naturalWidth && prevHot && prevCh
      ? lerpBox(spriteBox(prevImg, prevHot, prevCh, p, sizePx), curBox, m)
      : curBox;
  const tw = box[2],
    th = box[3];
  const at = (q: [number, number], im: HTMLImageElement) =>
    ctx.drawImage(im, box[0] + q[0] - p[0], box[1] + q[1] - p[1], tw, th);
  const blit = (q: [number, number]) => at(q, img);
  const posedBlit = (q: [number, number], im = img) => {
    if (isIdentity(pose)) {
      at(q, im);
      return;
    }
    drawPosed(ctx, q, pose, (r) => at(r, im));
  };

  const last = c.recent[c.recent.length - 1];
  if (last && Math.hypot(p[0] - last[0], p[1] - last[1]) > outH * 0.2) c.recent.length = 0;
  c.recent.push(p);
  if (c.recent.length > 6) c.recent.shift();

  ctx.save();
  ctx.beginPath();
  ctx.rect(clip[0], clip[1], clip[2] - clip[0], clip[3] - clip[1]);
  ctx.clip();
  if (c.back === "glass") {
    drawBack(ctx, [box[0] + tw / 2, box[1] + th / 2], kind, morph.prev, morph.m, th);
  }
  if (c.motionBlur > 0) {
    const trail = c.recent.slice(0, -1).reverse();
    let lastP = p;
    for (let i = 0; i < trail.length; i++) {
      const dist = Math.hypot(trail[i][0] - lastP[0], trail[i][1] - lastP[1]);
      if (dist < 1.5) continue;
      ctx.globalAlpha = Math.min(1, c.motionBlur * (1 - i / trail.length) * 0.5) * (glass ? GLASS_ALPHA : 1);
      blit(trail[i]);
      lastP = trail[i];
    }
    ctx.globalAlpha = 1;
  }
  if (glass && m < 1 && prevImg) {
    ctx.globalAlpha = GLASS_ALPHA * (1 - m);
    posedBlit(p, prevImg);
  }
  if (glass) ctx.globalAlpha = GLASS_ALPHA * m;
  posedBlit(p);
  ctx.globalAlpha = 1;
  ctx.restore();
}

function drawPosed(
  ctx: CanvasRenderingContext2D,
  q: [number, number],
  pose: BusyPose,
  blit: (q: [number, number]) => void,
) {
  ctx.save();
  ctx.translate(q[0], q[1]);
  ctx.rotate((pose.angleDeg * Math.PI) / 180);
  ctx.scale(pose.scale, pose.scale);
  ctx.translate(-q[0], -q[1]);
  blit(q);
  ctx.restore();
}
