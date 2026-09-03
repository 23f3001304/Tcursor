import type { CursorKindSample, ClickSample } from "../../lib/ipc";

/** Everything the preview needs to draw the export cursor: the recording's cursor style/size +
 *  bounce, the type track, and the decoded sprite images (+ hotspots + canvas heights) by kind. */
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
  recent: [number, number][];
}

/** The cursor type active at output time `ms` (last sample with t <= ms), default "arrow". */
export function cursorAt(kinds: CursorKindSample[], ms: number): string {
  let lo = 0, hi = kinds.length;
  while (lo < hi) { const mid = (lo + hi) >> 1; if (kinds[mid].t <= ms) lo = mid + 1; else hi = mid; }
  return lo > 0 ? kinds[lo - 1].kind : "arrow";
}

/** Draw the real export cursor sprite at the mapped position `p`, gated by style: only when
 *  "enhanced" (Hidden = none; System = the OS cursor is already in the video, EXCEPT on a
 *  recording that baked none - Stage passes an effective "enhanced" there, see its plain-OS
 *  fallback, which is why this gate stays a plain style check). Sized like the
 *  export (size * outH * 0.033 * `panel`, uniform on the sprite's canvas height), anchored at the
 *  hotspot, with the same post-click bounce dip and a fading motion trail (driven by
 *  `motionBlur`) - clipped to `clip`, both mirroring `cursorset::draw`'s panel scale + clip (see
 *  `cursorPanel.ts`). `clip` is post-zoom canvas px `[x0, y0, x1, y1]`, possibly inverted (x0 past
 *  x1) when the panel falls entirely outside the current zoom crop - the no-op check below is the
 *  same guard `blit`'s `ox_start >= ox_end` provides in Rust. */
export function drawCursorSprite(
  ctx: CanvasRenderingContext2D,
  p: [number, number],
  now: number,
  c: DrawCursor,
  clicks: ClickSample[],
  outH: number,
  panel: number,
  clip: [number, number, number, number],
) {
  if (c.style !== "enhanced") return;
  if (clip[2] <= clip[0] || clip[3] <= clip[1]) return; // panel entirely outside the crop
  const kind = cursorAt(c.kinds, now);
  const img = c.sprites.get(kind) ?? c.sprites.get("arrow");
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
  blit(p);
  ctx.restore();
}
