import { spring, SPRING_DEFAULT } from "../../lib/spring";

/** The six named curves as small glyphs for the camera lane's transition popover (the compact
 *  buttons between two keyframes). `key` is the wire name the ops and the shared `ease` understand;
 *  `path` depicts what the curve evaluates to, the spring one sampled from the real oscillator so
 *  the glyph cannot lie. The inspectors use the plain `EasingPicker` dropdown instead. */
export interface CurveGlyph { key: string; name: string; path: string }

const SAMPLES = 48;
const r2 = (v: number) => Math.round(v * 100) / 100;

function springPath(stiffness: number, damping: number): string {
  const pts: string[] = [];
  for (let i = 1; i <= SAMPLES; i++) {
    const p = i / SAMPLES;
    pts.push(`${r2(p * 100)} ${r2((1 - spring(stiffness, damping, 1, p)) * 100)}`);
  }
  return `M 0 100 L ${pts.join(" L ")}`;
}

export const CURVE_GLYPHS: CurveGlyph[] = [
  { key: "linear", name: "Linear", path: "M 0 100 L 100 0" },
  { key: "ease_in", name: "Ease in", path: "M 0 100 C 55 98, 82 55, 100 0" },
  { key: "ease_out", name: "Ease out", path: "M 0 100 C 18 45, 45 2, 100 0" },
  { key: "ease_in_out", name: "Ease in and out", path: "M 0 100 C 45 100, 55 0, 100 0" },
  { key: "smooth", name: "Smooth", path: "M 0 100 C 35 100, 65 0, 100 0" },
  { key: "spring", name: "Spring", path: springPath(SPRING_DEFAULT[0], SPRING_DEFAULT[1]) },
];
