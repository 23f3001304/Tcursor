import type { ModeAppearance } from "../settings/settings";
import type { ModeKey } from "../preferences/appearanceFields";

type Box = { left: number; top: number; width: number; height: number };

function clamp(v: number, lo: number, hi: number) { return Math.max(lo, Math.min(hi, v)); }

function camBorderRadius(ma: ModeAppearance): string {
  if (ma.cam_shape === "circle") return "50%";
  if (ma.cam_shape === "rounded") return `${ma.cam_radius * 100}%`;
  return "0";
}

/** Returns [screenBox, camBox | null] as fractions of stage (0..1).
 *  Stage is 16:9. A box with equal width-frac and height-frac is visually 16:9. */
function compose(mode: ModeKey, ma: ModeAppearance): [Box | null, Box | null] {
  const p = ma.pad;

  if (mode === "screen" || mode === "screen_only") {
    // Screen: width-frac == height-frac on a 16:9 stage => visually 16:9
    const base = 1 - 2 * p;
    const sw = clamp(base * ma.screen_size, 0, 1);
    const sh = sw; // equal fracs on 16:9 stage = 16:9 box
    const sl = clamp(0.5 - sw / 2, 0, 1);
    const st = clamp(0.5 - sh / 2, 0, 1);
    const screen: Box = { left: sl, top: st, width: sw, height: sh };

    if (mode === "screen_only") return [screen, null];

    // Cam bubble: cam_size as fraction of stage height; convert to both axes
    const cs = clamp(ma.cam_size, 0.05, 0.4); // height-frac
    const cw = cs * (9 / 16); // width-frac (narrower on 16:9 stage)
    const ch = cs;
    const mx = ma.cam_margin_x;
    const my = ma.cam_margin_y;
    let cl = 0, ct = 0;
    const c = ma.cam_corner;
    if (c === "bottom_left")  { cl = mx;          ct = clamp(1 - my - ch, 0, 1); }
    if (c === "bottom_right") { cl = clamp(1 - mx - cw, 0, 1); ct = clamp(1 - my - ch, 0, 1); }
    if (c === "top_left")     { cl = mx;           ct = my; }
    if (c === "top_right")    { cl = clamp(1 - mx - cw, 0, 1); ct = my; }
    const cam: Box = { left: clamp(cl, 0, 1), top: clamp(ct, 0, 1), width: cw, height: ch };
    return [screen, cam];
  }

  if (mode === "camera" || mode === "camera_only") {
    // Cam: large centered; cam_size as height-frac
    const ch = clamp(ma.cam_size, 0, 1 - 2 * p);
    const cw = ch * (9 / 16);
    const cam: Box = {
      left: clamp(0.5 - cw / 2, 0, 1), top: clamp(p, 0, 1),
      width: cw, height: ch,
    };

    if (mode === "camera_only") return [null, cam];

    // Screen inset: small 16:9 box bottom-left
    const sw = 0.28;
    const sh = sw; // equal fracs = 16:9
    const scr: Box = { left: p, top: clamp(1 - p - sh, 0, 1), width: sw, height: sh };
    return [scr, cam];
  }

  // presenter: cam left half, screen right half
  const half = 0.5 - p * 1.5;
  const camH = clamp(1 - 2 * p, 0, 1);
  const camW = camH * (9 / 16);
  const camLeft = clamp(p + (half - camW) / 2, 0, 1);
  const cam: Box = { left: camLeft, top: p, width: camW, height: camH };
  const scrW = clamp(half, 0, 1);
  const scrH = scrW; // equal fracs = 16:9 on 16:9 stage
  const scrLeft = clamp(0.5 + p / 2, 0, 1);
  const scrTop = clamp(0.5 - scrH / 2, 0, 1);
  const screen: Box = { left: scrLeft, top: scrTop, width: scrW, height: scrH };
  return [screen, cam];
}

function toStyle(b: Box): React.CSSProperties {
  return {
    position: "absolute",
    left: `${b.left * 100}%`,
    top: `${b.top * 100}%`,
    width: `${b.width * 100}%`,
    height: `${b.height * 100}%`,
  };
}

export function LayoutPreview({ mode, ma }: { mode: ModeKey; ma: ModeAppearance }) {
  const [screen, cam] = compose(mode, ma);
  const scrRadius = `${ma.screen_radius * 100}%`;

  return (
    <div className="lp-stage">
      {screen && <div className="lp-screen" style={{ ...toStyle(screen), borderRadius: scrRadius }} />}
      {cam && <div className="lp-cam" style={{ ...toStyle(cam), borderRadius: camBorderRadius(ma) }} />}
    </div>
  );
}
