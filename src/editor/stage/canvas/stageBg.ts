import type { BackgroundKind } from "../../../hud/settings/settings";
import type { GifFrames } from "./gifFrames";
import { gifIndexAt } from "./gifFrames";

export interface StageBg {
  url: string;
  assetUrl: string;
  assetPath: string;
  kind: BackgroundKind;
  dim: number;
}

export interface StageBgState {
  bg: StageBg;
  img: HTMLImageElement | null;
  video: HTMLVideoElement | null;
  gif: GifFrames | null;
  playing: boolean;
}

const DRIFT_MS = 150;

export function coverRect(sw: number, sh: number, dw: number, dh: number): [number, number, number, number] {
  if (sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0) return [0, 0, sw, sh];
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale,
    ch = dh / scale;
  return [(sw - cw) / 2, (sh - ch) / 2, cw, ch];
}

export function loopMs(tMs: number, durMs: number): number {
  if (!(durMs > 0) || !(tMs > 0)) return 0;
  return tMs % durMs;
}

export function isGif(assetPath: string): boolean {
  return assetPath.toLowerCase().endsWith(".gif");
}

export function bgAssetUrl(
  folder: string,
  asset: string | null | undefined,
  kind: BackgroundKind,
  toSrc: (p: string) => string,
): string {
  if (!asset || (kind !== "image" && kind !== "video")) return "";
  return toSrc(`${folder}\\${asset.split("/").join("\\")}`);
}

export function drawBackground(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  st: StageBgState | null,
  tMs: number,
) {
  if (st && st.bg.kind === "video" && drawMoving(ctx, w, h, st, tMs)) {
    const dim = Math.min(Math.max(st.bg.dim, 0), 0.8);
    if (dim > 0) {
      ctx.fillStyle = `rgba(0, 0, 0, ${dim})`;
      ctx.fillRect(0, 0, w, h);
    }
    return;
  }
  const img = st?.img;
  if (img && img.complete && img.naturalWidth > 0) {
    ctx.drawImage(img, 0, 0, w, h);
    return;
  }
  const g = ctx.createLinearGradient(0, 0, w * 0.4, h);
  g.addColorStop(0, "#2c2c42");
  g.addColorStop(1, "#131318");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
}

function drawMoving(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  st: StageBgState,
  tMs: number,
): boolean {
  const { gif, video } = st;
  if (gif && gif.frames.length) {
    const f = gif.frames[gifIndexAt(gif.ends, tMs)];
    ctx.drawImage(f, ...coverRect(f.width, f.height, w, h), 0, 0, w, h);
    return true;
  }
  if (!video || video.readyState < 2 || !video.videoWidth) return false;
  const want = loopMs(tMs, (video.duration || 0) * 1000);
  if (!st.playing || Math.abs(video.currentTime * 1000 - want) > DRIFT_MS) video.currentTime = want / 1000;
  ctx.drawImage(video, ...coverRect(video.videoWidth, video.videoHeight, w, h), 0, 0, w, h);
  return true;
}
