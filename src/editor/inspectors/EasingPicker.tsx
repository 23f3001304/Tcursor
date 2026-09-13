import { parseCubic } from "../../lib/cubicBezier";
import { springOf } from "../../lib/spring";
import { CURVE_GLYPHS } from "../timeline/curveGlyphs";
import { SegRow } from "./InspectorShape";

/** The option a stored easing string selects: a parameterised `spring(...)` is the Spring option,
 *  a `cubic(...)` (what dragging a handle writes) is Custom. */
export function easingOption(value: string): string {
  if (springOf(value)) return "spring";
  if (parseCubic(value)) return "custom";
  return CURVE_GLYPHS.some((c) => c.key === value) ? value : "smooth";
}

/** The segmented row of named curves that sits above `CurveEditor`'s canvas. Names and keys come
 *  from `timeline/curveGlyphs.ts` so the row, the timeline's transition popover and the export all
 *  read one list. A seventh Custom segment appears only while the value is a cubic; picking it
 *  again is a no-op (there is nothing to switch back TO - the canvas' handles are how you leave
 *  it), so it is rendered selected-and-inert rather than as a choice. */
export function EasingPicker({ value, onChange, label = "Transition curve" }: {
  value: string; onChange: (easing: string) => void; label?: string;
}) {
  const option = easingOption(value);
  const options = [
    ...CURVE_GLYPHS.map((c) => ({ key: c.key, label: c.name, on: option === c.key })),
    ...(option === "custom" ? [{ key: "custom", label: "Custom", on: true }] : []),
  ];
  return (
    <div className="e-field e-curve-row">
      <span className="e-fl">{label}</span>
      <SegRow ariaLabel={label} options={options}
        onPick={(k) => { if (k !== "custom" && k !== option) onChange(k); }} />
    </div>
  );
}
