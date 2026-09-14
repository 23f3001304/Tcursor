// The glass cursor material, as much of it as a 2D canvas can do. Split out of `cursorPreview.ts`
// for its size budget; the export's own version is Rust `export/fx/fx_lens.rs` + `fx_lens.wgsl`,
// and every number below is that file's.
//
// DELIBERATE DIFFERENCES from the export, all of them "less, never elsewhere": the back is a plain
// magnifier (the canvas re-drawn onto itself `LENS_ZOOM` bigger - the readable part of the glass)
// with no rim bend, no frost and no rim light; the sprite-shaped lens of a glass pack does not
// magnify at all (a 2D canvas cannot clip to a sprite's alpha in one pass); no drop shadow, no
// click squash, no ink drop or over-text ring, and the back does not stretch along a text
// selection. What DOES match is every position, size and shape, including the state cross-fade:
// scrubbing to a pause swaps in the backend's exact frame, and nothing may MOVE when it does - the
// glass simply resolves.
import type { CursorKindSample } from "../../lib/ipc";

/** The alpha a glass pack's sprite is drawn at, mirroring Rust `fx_lens::SPRITE_ALPHA`. */
export const GLASS_ALPHA = 0.65;
/** The magnification inside a glass shape - Rust `fx_lens::ZOOM` / `fx_lens.wgsl::LENS_ZOOM`. */
export const LENS_ZOOM = 1.35;
/** The back's diameter as a multiple of the sprite's drawn height - Rust `fx_lens::BACK_SCALE`. */
export const BACK_SCALE = 2.2;
/** The back's height over text as a fraction of its width - Rust `fx_lens::PILL_W`. */
export const PILL_W = 0.35;
/** How long a cursor state change eases over, in ms - Rust `fx_lens::MORPH_MS`. */
export const MORPH_MS = 160;

/** The click effects' ease-out cubic, which the morph shares - Rust `clickfx::ease_out`, mirrored
 *  by `ripplePreview.ts::easeOut` and `fx_clicks.wgsl::fx_ease`. */
export function morphEase(p: number): number {
  const q = 1 - Math.min(1, Math.max(0, p));
  return 1 - q * q * q;
}

/** One cursor state change in flight at output time `ms`: the kind now in force, the one before it,
 *  and how far the change has eased through `MORPH_MS` (1 = settled). The TS mirror of Rust
 *  `fx_lens::kind_morph`, down to "an empty track is a settled arrow, never a morph from nothing". */
export function cursorMorphAt(kinds: CursorKindSample[], ms: number): { kind: string; prev: string; m: number } {
  let lo = 0, hi = kinds.length;
  while (lo < hi) { const mid = (lo + hi) >> 1; if (kinds[mid].t <= ms) lo = mid + 1; else hi = mid; }
  if (lo === 0) return { kind: "arrow", prev: "arrow", m: 1 };
  const at = kinds[lo - 1];
  return { kind: at.kind, prev: lo >= 2 ? kinds[lo - 2].kind : "arrow", m: morphEase((ms - at.t) / MORPH_MS) };
}

/** A sprite's placed box in canvas px as `[x0, y0, w, h]` - the hotspot lands on `p`. The mirror of
 *  Rust `cursormorph::sprite_box`. */
export function spriteBox(img: HTMLImageElement, hot: [number, number], canvasH: number,
  p: [number, number], sizePx: number): [number, number, number, number] {
  const scale = sizePx / Math.max(canvasH, 1);
  const w = img.naturalWidth * scale, h = img.naturalHeight * scale;
  return [p[0] - hot[0] * w, p[1] - hot[1] * h, w, h];
}

/** `a` lerped to `b` by an already-eased `m` - Rust `cursormorph::lerp_box`. */
export function lerpBox(a: [number, number, number, number], b: [number, number, number, number],
  m: number): [number, number, number, number] {
  const t = Math.min(1, Math.max(0, m));
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t,
          a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t];
}

/** The cursor back's half-extents for one cursor kind, given the sprite's DRAWN height in canvas
 *  px: a disc for every shape but the I-beam, which is a HORIZONTAL pill `PILL_W` as tall (a line
 *  of text is horizontal, and so is the selection it would stretch along). Rust `fx_lens::back_of`.
 *  The corner radius is always the shorter half-extent, which is what makes one rounded rect serve
 *  as both circle and pill. */
export function backBox(kind: string, spriteH: number): { rx: number; ry: number } {
  const r = (spriteH * BACK_SCALE) / 2;
  return kind === "ibeam" ? { rx: r, ry: r * PILL_W } : { rx: r, ry: r };
}

/** The cursor back for the live canvas: a magnifying rounded rect, morphing between kinds on the
 *  same eased progress the export uses, centred on the sprite's BOX (an arrow's hotspot is its tip,
 *  so a disc centred there would sit up and left of the cursor it is behind). The magnifier is the
 *  canvas drawn onto itself: the shape's box shrunk by `1/LENS_ZOOM` about the centre, stretched
 *  back over the whole shape - `drawImage` snapshots its source first, so the canvas IS the
 *  undisturbed frame the export re-samples. Drawn before the trail and the sprite, so they land on
 *  top of the glass exactly as the export blits them over its refraction. */
export function drawBack(ctx: CanvasRenderingContext2D, centre: [number, number],
  kind: string, prev: string, m: number, spriteH: number) {
  const a = backBox(prev, spriteH), b = backBox(kind, spriteH);
  const t = Math.min(1, Math.max(0, m));
  const rx = a.rx + (b.rx - a.rx) * t, ry = a.ry + (b.ry - a.ry) * t;
  if (rx < 0.5 || ry < 0.5) return;
  ctx.save();
  ctx.beginPath();
  ctx.roundRect(centre[0] - rx, centre[1] - ry, rx * 2, ry * 2, Math.min(rx, ry));
  ctx.clip();
  const sw = (rx * 2) / LENS_ZOOM, sh = (ry * 2) / LENS_ZOOM;
  ctx.drawImage(ctx.canvas, centre[0] - sw / 2, centre[1] - sh / 2, sw, sh, centre[0] - rx, centre[1] - ry, rx * 2, ry * 2);
  // The shader's ~8% lift + cool cast is a multiply; a faint wash is the nearest a canvas fill gets
  // without dimming the letters the magnifier just made readable.
  ctx.fillStyle = "rgba(235, 243, 255, 0.10)";
  ctx.fill();
  // The rim the shader paints (`fx_lens.wgsl::BACK_RIM`), without its light direction - a flat
  // stroke is what a fill-only disc is missing to read as an object rather than a smudge.
  ctx.strokeStyle = "rgba(255, 255, 255, 0.30)";
  ctx.lineWidth = 1;
  ctx.stroke();
  ctx.restore();
}
