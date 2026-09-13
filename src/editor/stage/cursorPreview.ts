import type { CursorKindSample, ClickSample } from "../../lib/ipc";
import { busyPose, isIdentity, STILL, type BusyPose, type BusySpec } from "./cursorBusy";

/** The recording's captured OS-cursor layer, decoded: the `[t, id]` timeline, each real cursor
 *  bitmap and its hotspot in captured pixels, and the recorded video's own width (`srcW`) - the
 *  bitmaps are in SOURCE pixels, so `contentScale` needs it to size them relative to the screen
 *  content. Mirrors Rust `export::cursor::captured`. */
export interface CapturedLayer {
  track: [number, number][];
  images: Map<number, HTMLImageElement>;
  hots: Map<number, [number, number]>;
  srcW: number;
}

/** Everything the preview needs to draw the export cursor: the recording's cursor style/size +
 *  bounce, the type track, the decoded sprite images (+ hotspots + canvas heights) by kind, and
 *  the captured OS-cursor layer when this recording has one. */
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
  /** The selected pack's busy animation (pack format v2), null for a still one. */
  busy: BusySpec | null;
  /** Decoded `busy_NN.png` frames when the pack ships them, empty otherwise. Indexed by
   *  `BusyPose.frame`, so an out-of-range index falls back to `sprites`' own busy image. */
  busyFrames: HTMLImageElement[];
  recent: [number, number][];
}

/** The captured cursor id showing at output time `ms` (last sample with t <= ms), or null before
 *  the first sample. The TS mirror of Rust `CursorLayer::id_at`. */
export function idAt(track: [number, number][], ms: number): number | null {
  let lo = 0, hi = track.length;
  while (lo < hi) { const mid = (lo + hi) >> 1; if (track[mid][0] <= ms) lo = mid + 1; else hi = mid; }
  return lo > 0 ? track[lo - 1][1] : null;
}

/** Draw the REAL recorded cursor bitmap, hotspot on the raw recorded point, at `scale` (canvas px
 *  per source px - `cursorPanel.ts`'s `contentScale`) - the mirror of Rust
 *  `CapturedCursors::draw`. No bounce, no trail, no glide: "System" is the cursor that was
 *  actually on screen, not an idealized one. */
function drawCaptured(ctx: CanvasRenderingContext2D, p: [number, number], now: number,
  cap: CapturedLayer, scale: number, clip: [number, number, number, number]) {
  const id = idAt(cap.track, now);
  if (id === null) return;
  const img = cap.images.get(id);
  if (!img || !img.complete || !img.naturalWidth) return;
  const hot = cap.hots.get(id) ?? [0, 0];
  ctx.save();
  ctx.beginPath();
  ctx.rect(clip[0], clip[1], clip[2] - clip[0], clip[3] - clip[1]);
  ctx.clip();
  ctx.drawImage(img, p[0] - hot[0] * scale, p[1] - hot[1] * scale,
    img.naturalWidth * scale, img.naturalHeight * scale);
  ctx.restore();
}

/** The cursor type active at output time `ms` (last sample with t <= ms), default "arrow". */
export function cursorAt(kinds: CursorKindSample[], ms: number): string {
  let lo = 0, hi = kinds.length;
  while (lo < hi) { const mid = (lo + hi) >> 1; if (kinds[mid].t <= ms) lo = mid + 1; else hi = mid; }
  return lo > 0 ? kinds[lo - 1].kind : "arrow";
}

/** Draw the export cursor at the mapped position `p`. A captured layer (`c.captured`, populated
 *  by Stage only for "system" on a recording that has one) draws the REAL recorded bitmap and
 *  returns; otherwise the synthetic sprite is gated by style: only when "enhanced" (Hidden =
 *  none; System = the OS cursor is already in the video, EXCEPT on a pre-layer recording that
 *  baked none - Stage passes an effective "enhanced" there, see its plain-OS fallback, which is
 *  why that gate stays a plain style check). Sized like the
 *  export (size * outH * 0.033 * `panel`, uniform on the sprite's canvas height), anchored at the
 *  hotspot, with the same post-click bounce dip and a fading motion trail (driven by
 *  `motionBlur`) - clipped to `clip`, both mirroring `cursorset::draw`'s panel scale + clip (see
 *  `cursorPanel.ts`). `clip` is post-zoom canvas px `[x0, y0, x1, y1]`, possibly inverted (x0 past
 *  x1) when the panel falls entirely outside the current zoom crop - the no-op check below is the
 *  same guard `blit`'s `ox_start >= ox_end` provides in Rust. `capturedScale` is canvas px per
 *  SOURCE px (`cursorPanel.ts`'s `contentScale`), used by the captured path only - the recorded
 *  bitmaps are in source pixels, so `panel` alone would size them by the wrong basis. */
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
  if (clip[2] <= clip[0] || clip[3] <= clip[1]) return; // panel entirely outside the crop
  // A recording with a captured layer draws the real cursor for "System" (Stage only populates
  // `captured` in that case) - the synthetic path below stays Enhanced-only, as before.
  if (c.captured) { drawCaptured(ctx, p, now, c.captured, capturedScale, clip); return; }
  if (c.style !== "enhanced") return;
  const kind = cursorAt(c.kinds, now);
  // Pack v2: the busy state may animate. `now` is the OUTPUT clock, the same basis the export
  // passes `busy_pose`, so a paused preview shows exactly the frame the export would write.
  const pose = kind === "busy" && c.busy ? busyPose(c.busy, now) : STILL;
  const frame = c.busyFrames[pose.frame];
  const img = frame ?? c.sprites.get(kind) ?? c.sprites.get("arrow");
  const ch = c.canvasH.get(kind) ?? c.canvasH.get("arrow");
  const hot = c.hots.get(kind) ?? c.hots.get("arrow");
  if (!img || !img.complete || !img.naturalWidth || !ch || !hot) return;
  const clampedSize = Math.min(Math.max(c.size, 0.4), 3.0);
  // The cursor does NOT scale with the camera zoom (only its position moves, mapped in by the
  // caller before `p` reaches here) - `panel` is a LAYOUT factor (the screen panel's own size),
  // not a zoom factor, exactly like the export's `size_px = ... * panel` in cursordraw.rs.
  let sizePx = clampedSize * outH * 0.033 * panel;
  if (c.clickBounce) {
    for (let i = clicks.length - 1; i >= 0; i--) {
      const dt = now - clicks[i].t;
      if (dt < 0) continue;
      if (dt < 180) sizePx *= 1 - 0.36 * c.bounceIntensity * (1 - dt / 180);
      break;
    }
  }
  const scale = sizePx / ch;
  const tw = img.naturalWidth * scale, th = img.naturalHeight * scale;
  const blit = (q: [number, number]) => ctx.drawImage(img, q[0] - hot[0] * tw, q[1] - hot[1] * th, tw, th);
  // An explicit frame is already the animation, so it is only ever drawn still (`busyPose`
  // returns the identity for it) - the transform below is for a synthesised spin/pulse.
  const posedBlit = (q: [number, number]) => {
    if (isIdentity(pose)) { blit(q); return; }
    drawPosed(ctx, q, pose, blit);
  };
  
  const last = c.recent[c.recent.length - 1];
  if (last && Math.hypot(p[0] - last[0], p[1] - last[1]) > outH * 0.2) c.recent.length = 0;
  c.recent.push(p); if (c.recent.length > 6) c.recent.shift();

  // Confine the cursor + its trail to the screen panel's on-screen rect (post-zoom), exactly like
  // the export's `blit` clip - so a shrunk custom-arrangement panel's cursor never spills onto the
  // background or the webcam.
  ctx.save();
  ctx.beginPath();
  ctx.rect(clip[0], clip[1], clip[2] - clip[0], clip[3] - clip[1]);
  ctx.clip();
  if (c.motionBlur > 0) {
    const trail = c.recent.slice(0, -1).reverse();
    let lastP = p;
    for (let i = 0; i < trail.length; i++) {
      const dist = Math.hypot(trail[i][0] - lastP[0], trail[i][1] - lastP[1]);
      if (dist < 1.5) continue;
      ctx.globalAlpha = Math.min(1, c.motionBlur * (1 - i / trail.length) * 0.5);
      blit(trail[i]);
      lastP = trail[i];
    }
    ctx.globalAlpha = 1;
  }
  posedBlit(p);
  ctx.restore();
}

/** Run `blit` under the busy pose's rotation/scale about the cursor point `q` - the canvas mirror
 *  of Rust `cursorxform::blit_transformed`, which rotates about the sprite's hotspot and leaves it
 *  on the anchor. Same anchor as the untransformed draw, so an animating busy cursor never drifts
 *  off the cursor point. */
function drawPosed(ctx: CanvasRenderingContext2D, q: [number, number], pose: BusyPose,
  blit: (q: [number, number]) => void) {
  ctx.save();
  ctx.translate(q[0], q[1]);
  ctx.rotate((pose.angleDeg * Math.PI) / 180);
  ctx.scale(pose.scale, pose.scale);
  ctx.translate(-q[0], -q[1]);
  blit(q);
  ctx.restore();
}
