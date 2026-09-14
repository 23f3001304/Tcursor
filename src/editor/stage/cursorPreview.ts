import type { CursorKindSample, ClickSample } from "../../lib/ipc";
import { busyPose, isIdentity, STILL, type BusyPose, type BusySpec } from "./cursorBusy";
import { cursorMorphAt, drawBack, lerpBox, spriteBox, GLASS_ALPHA } from "./cursorGlass";

/** The recording's captured OS-cursor layer, decoded: the `[t, id]` timeline, each bitmap and
 *  hotspot in captured pixels, and the video's own width (`srcW`, which `contentScale` sizes the
 *  SOURCE-pixel bitmaps against). Mirrors Rust `export::cursor::captured`. */
export interface CapturedLayer {
  track: [number, number][];
  images: Map<number, HTMLImageElement>;
  hots: Map<number, [number, number]>; srcW: number;
}

/** Everything the preview needs to draw the export cursor: style, size and bounce, the type track,
 *  the decoded sprites (hotspots, canvas heights) by kind, and the captured layer if any. */
export interface DrawCursor {
  style: string; size: number;
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
  /** Decoded `busy_NN.png` frames if the pack ships them (by `BusyPose.frame`; out of range falls back to `sprites`). */
  busyFrames: HTMLImageElement[];
  /** The pack's `material`: `"glass"` = lenses (Rust `fx_lens`), drawn at `GLASS_ALPHA` like the export. */
  material: string | null;
  /** This frame's motion lean in degrees (`cursorTilt.ts`, driven by `useCompositeLoop`), composed
   *  with whatever rotation the busy pose already carries - Rust `cursorset::draw`'s `tilt_deg`. */
  tiltDeg: number;
  /** The cursor back setting (`CursorSettings.back`) - `"glass"` draws the disc/pill. */
  back: string;
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

/** Draw the export cursor at the mapped position `p`. A captured layer (`c.captured`, set by Stage
 *  only for "system" on a recording that has one) draws the REAL recorded bitmap and returns;
 *  otherwise the synthetic sprite draws only when "enhanced" (Hidden = none; System = already in
 *  the video, except a pre-layer recording that baked none, where Stage passes an effective
 *  "enhanced"). Sized like the export (size * outH * 0.033 * `panel`, uniform on the sprite's
 *  canvas height), anchored at the hotspot, with the post-click bounce dip, the `motionBlur` trail
 *  and the `tiltDeg` lean, clipped to `clip` (post-zoom canvas px `[x0, y0, x1, y1]`, possibly
 *  inverted when the panel is outside the zoom crop - the same no-op guard as `blit`'s in Rust),
 *  mirroring `cursorset::draw`. `capturedScale` (canvas px per SOURCE px, `cursorPanel.ts`'s
 *  `contentScale`) sizes the captured bitmaps, which are in source pixels. */
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
  // A glass pack's states MORPH into each other (its shapes are objects - a disc, a pill - where a
  // snap reads as a swap); a plain pack's are pictures of pointers, where a snap is what an OS
  // cursor does. `m` is 1 (settled) for everything but a glass pack mid-change.
  const glass = c.material === "glass";
  const morph = cursorMorphAt(c.kinds, now);
  const kind = morph.kind;
  const m = glass ? morph.m : 1;
  // Pack v2: the busy state may animate. `now` is the OUTPUT clock, the same basis the export
  // passes `busy_pose`, so a paused preview shows exactly the frame the export would write. The
  // motion lean rides on the same rotation, about the same hotspot, exactly as in the export.
  const spin = kind === "busy" && c.busy ? busyPose(c.busy, now) : STILL;
  const pose = c.tiltDeg ? { ...spin, angleDeg: spin.angleDeg + c.tiltDeg } : spin;
  const frame = c.busyFrames[spin.frame];
  const img = frame ?? c.sprites.get(kind) ?? c.sprites.get("arrow");
  const ch = c.canvasH.get(kind) ?? c.canvasH.get("arrow");
  const hot = c.hots.get(kind) ?? c.hots.get("arrow");
  if (!img || !img.complete || !img.naturalWidth || !ch || !hot) return;
  const clampedSize = Math.min(Math.max(c.size, 0.4), 3.0);
  // The cursor does NOT scale with the camera zoom (only its position moves, mapped in by the
  // caller): `panel` is a LAYOUT factor - the export's `size_px = ... * panel` in cursordraw.rs.
  let sizePx = clampedSize * outH * 0.033 * panel;
  if (c.clickBounce) {
    for (let i = clicks.length - 1; i >= 0; i--) {
      const dt = now - clicks[i].t;
      if (dt < 0) continue;
      if (dt < 180) sizePx *= 1 - 0.36 * c.bounceIntensity * (1 - dt / 180);
      break;
    }
  }
  // The placed box, interpolated from the outgoing state's while a glass morph is in flight (Rust
  // `cursormorph::morph_box`) - the box BOTH sprites stretch into, so the shapes dissolve through
  // each other instead of one sliding out from behind the other.
  const curBox = spriteBox(img, hot, ch, p, sizePx);
  const prevImg = m < 1 ? c.sprites.get(morph.prev) : undefined;
  const prevHot = c.hots.get(morph.prev), prevCh = c.canvasH.get(morph.prev);
  const box = prevImg && prevImg.complete && prevImg.naturalWidth && prevHot && prevCh
    ? lerpBox(spriteBox(prevImg, prevHot, prevCh, p, sizePx), curBox, m) : curBox;
  const tw = box[2], th = box[3];
  const at = (q: [number, number], im: HTMLImageElement) =>
    ctx.drawImage(im, box[0] + q[0] - p[0], box[1] + q[1] - p[1], tw, th);
  const blit = (q: [number, number]) => at(q, img);
  // An explicit frame is already the animation, so `busyPose` returns the identity for it - the
  // transform is then the motion lean alone, or nothing at all when the cursor is upright.
  const posedBlit = (q: [number, number], im = img) => {
    if (isIdentity(pose)) { at(q, im); return; }
    drawPosed(ctx, q, pose, (r) => at(r, im));
  };

  const last = c.recent[c.recent.length - 1];
  if (last && Math.hypot(p[0] - last[0], p[1] - last[1]) > outH * 0.2) c.recent.length = 0;
  c.recent.push(p); if (c.recent.length > 6) c.recent.shift();

  // Confine the cursor + its trail to the screen panel's on-screen rect (post-zoom), exactly like
  // the export's `blit` clip - a shrunk panel's cursor never spills onto the background or webcam.
  ctx.save(); ctx.beginPath();
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
  // A glass pack's sprite is the LENS's edge, not the picture: the export blits it at
  // `fx_lens::SPRITE_ALPHA` over live refraction this canvas cannot do (see `cursorGlass.ts`), so
  // matching the alpha and cross-fading the outgoing state under it is what keeps the moving
  // preview close to the paused exact frame the backend renders.
  if (glass && m < 1 && prevImg) {
    ctx.globalAlpha = GLASS_ALPHA * (1 - m);
    posedBlit(p, prevImg); // the outgoing state leans too - Rust `cursormorph::draw_glass`
  }
  if (glass) ctx.globalAlpha = GLASS_ALPHA * m;
  posedBlit(p);
  ctx.globalAlpha = 1;
  ctx.restore();
}

/** Run `blit` under the pose's rotation/scale about the cursor point `q` - the canvas mirror of
 *  Rust `cursorxform::blit_transformed`, which rotates about the sprite's hotspot and leaves it on
 *  the anchor. Same anchor as the untransformed draw, so a spinning or leaning cursor never drifts
 *  off the point it is pointing at. */
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
