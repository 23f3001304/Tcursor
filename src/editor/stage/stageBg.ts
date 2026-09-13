// The preview's background: what the stage is handed, what it holds, and how one frame of it is
// drawn.
//
// Two paths, and the split is the whole design. For everything STATIC (wallpaper, colour,
// gradient, an imported still) the backend already returns the finished background as a PNG at the
// output size, cover-fitted, blurred and DIMMED - so the preview just blits it, exactly as it
// always has, and re-applying anything here would double it. For a VIDEO or GIF the backend cannot
// hand over 60 images a second, so this file draws the moving asset itself: cover-fit with the
// same rect math the webcam PiP uses, then the dim as a black overlay - `out = src * (1 - dim)`,
// the same formula `background::apply_dim` uses in Rust, so preview and export agree.
//
// Parity for a moving background is "the same frame within one output frame", not pixel-identical:
// the preview resamples through the browser's decoder and the export through ffmpeg's `-r`.
import type { BackgroundKind } from "../../hud/settings/settings";
import type { GifFrames } from "./gifFrames";
import { gifIndexAt } from "./gifFrames";

/** What `Editor` hands the stage: the backend's finished background PNG, plus everything needed to
 *  draw a moving asset over it. One prop, so adding video did not widen `Stage`'s signature. */
export interface StageBg {
  /** `preview_bg`'s data URL - the static background, already dimmed by Rust. */
  url: string;
  /** The imported asset as an `asset://` URL, or `""` when the active kind does not draw one. */
  assetUrl: string;
  /** `settings.background.asset` (project-relative), for the `.gif` decision. */
  assetPath: string;
  kind: BackgroundKind;
  /** `settings.background.dim`, applied HERE only on the branch that paints pixels itself. */
  dim: number;
}

/** The live elements behind a `StageBg`, owned by `useStageInvalidation` and read every tick. */
export interface StageBgState {
  bg: StageBg;
  img: HTMLImageElement | null;
  /** A detached `<video muted loop playsInline>` - never mounted; `drawImage` needs no DOM node. */
  video: HTMLVideoElement | null;
  gif: GifFrames | null;
  /** Whether the transport is running, so a paused scrub seeks and playback does not fight it. */
  playing: boolean;
}

/** How far a drawn video may drift from the playhead before it is re-seeked (ms). Playback runs
 *  the element at 1x beside the screen video rather than seeking per frame, so some drift is
 *  normal; this is the same tolerance the webcam re-sync uses. */
const DRIFT_MS = 150;

/** The largest centred sub-rect of a `sw`x`sh` source with the destination's aspect: `[sx, sy, sw,
 *  sh]` for `drawImage`. Cover-fit - the mismatched axis is CROPPED, never squashed - so a 16:9
 *  loop behind a 9:16 export fills the frame instead of letterboxing. The same fit
 *  `ffio::decode_file_cover` gives a still and `bg_decode_args` asks ffmpeg for. */
export function coverRect(sw: number, sh: number, dw: number, dh: number): [number, number, number, number] {
  if (sw <= 0 || sh <= 0 || dw <= 0 || dh <= 0) return [0, 0, sw, sh];
  const scale = Math.max(dw / sw, dh / sh);
  const cw = dw / scale, ch = dh / scale;
  return [(sw - cw) / 2, (sh - ch) / 2, cw, ch];
}

/** The playhead wrapped into a loop of `durMs`. 0 when the duration is not known yet (a video
 *  whose metadata has not loaded), so the first frames show frame 0 rather than NaN - and 0 for a
 *  playhead before the clip starts, which is not a loop position at all: wrapping it backwards
 *  would answer "before the beginning" with the END of the loop. */
export function loopMs(tMs: number, durMs: number): number {
  if (!(durMs > 0) || !(tMs > 0)) return 0;
  return tMs % durMs;
}

/** Is this asset a GIF? The model calls a GIF a `video` (one export decode path), but the preview
 *  cannot play one in a `<video>`, so the decode path is chosen by extension. */
export function isGif(assetPath: string): boolean {
  return assetPath.toLowerCase().endsWith(".gif");
}

/** The asset's URL for the CURRENT kind, or `""`. The asset is deliberately kept in the settings
 *  while a wallpaper or colour is showing (so re-selecting it needs no re-import) - but it must
 *  not be loaded then, or a hidden `<video>` would sit there decoding a background nobody sees. */
export function bgAssetUrl(folder: string, asset: string | null | undefined, kind: BackgroundKind,
                           toSrc: (p: string) => string): string {
  if (!asset || (kind !== "image" && kind !== "video")) return "";
  // Stored forward-slashed (portable); the asset protocol wants this platform's separator.
  return toSrc(`${folder}\\${asset.split("/").join("\\")}`);
}

/** Draw the background for one preview frame into `ctx` at `w`x`h`. */
export function drawBackground(ctx: CanvasRenderingContext2D, w: number, h: number,
                               st: StageBgState | null, tMs: number) {
  if (st && st.bg.kind === "video" && drawMoving(ctx, w, h, st, tMs)) {
    // A moving background paints its own pixels, so the dim belongs here - once.
    const dim = Math.min(Math.max(st.bg.dim, 0), 0.8);
    if (dim > 0) { ctx.fillStyle = `rgba(0, 0, 0, ${dim})`; ctx.fillRect(0, 0, w, h); }
    return;
  }
  // Static: the exact export background, already dimmed in Rust. Until it lands (or if the video
  // above has no frame yet) the same gradient placeholder the preview has always used.
  const img = st?.img;
  if (img && img.complete && img.naturalWidth > 0) { ctx.drawImage(img, 0, 0, w, h); return; }
  const g = ctx.createLinearGradient(0, 0, w * 0.4, h);
  g.addColorStop(0, "#2c2c42"); g.addColorStop(1, "#131318");
  ctx.fillStyle = g; ctx.fillRect(0, 0, w, h);
}

/** One frame of a GIF or video, cover-fitted. `false` when there is nothing decoded yet, which
 *  makes the caller fall back to the still background rather than flashing a gap. */
function drawMoving(ctx: CanvasRenderingContext2D, w: number, h: number, st: StageBgState, tMs: number): boolean {
  const { gif, video } = st;
  if (gif && gif.frames.length) {
    const f = gif.frames[gifIndexAt(gif.ends, tMs)];
    ctx.drawImage(f, ...coverRect(f.width, f.height, w, h), 0, 0, w, h);
    return true;
  }
  if (!video || video.readyState < 2 || !video.videoWidth) return false;
  const want = loopMs(tMs, (video.duration || 0) * 1000);
  // Paused: seek to the playhead. Playing: let it run at 1x beside the screen video and only
  // correct real drift - seeking every frame would stall the decoder and stutter the picture.
  if (!st.playing || Math.abs(video.currentTime * 1000 - want) > DRIFT_MS) video.currentTime = want / 1000;
  ctx.drawImage(video, ...coverRect(video.videoWidth, video.videoHeight, w, h), 0, 0, w, h);
  return true;
}
