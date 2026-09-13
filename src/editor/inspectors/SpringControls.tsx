import { Slider } from "../controls/Slider";
import { formatSpring, SPRING_RANGE } from "../../lib/spring";

/** The two parameters that shape a spring, under `CurveEditor`'s canvas. A spring is NOT a cubic
 *  bezier, so no drag handle could express one - handles would only convert it to a cubic and lose
 *  the physics. The curve row picks Spring, the canvas draws the real sampled oscillator, and
 *  these two sliders edit that oscillator directly.
 *
 *  Only the DAMPING RATIO `c / (2*sqrt(k*m))` changes the drawn curve: the export remaps the
 *  response onto its own settle time, so raising stiffness and damping together is a no-op while
 *  raising either alone is not (see docs/api/src-tauri/src/export/spring.md). Mass is on the wire
 *  for completeness but is not exposed - it is redundant with stiffness for shaping. */
export function SpringControls({ stiffness, damping, mass, onChange }: {
  stiffness: number; damping: number; mass: number; onChange: (easing: string) => void;
}) {
  return (
    <div className="e-field2">
      <label className="e-field">
        <Slider min={SPRING_RANGE.stiffness[0]} max={SPRING_RANGE.stiffness[1]} step={1}
          value={stiffness} label="Stiffness" ariaLabel="Spring stiffness"
          formatValue={(v) => v.toFixed(0)}
          onChange={(v) => onChange(formatSpring(v, damping, mass))} />
      </label>
      <label className="e-field">
        <Slider min={SPRING_RANGE.damping[0]} max={SPRING_RANGE.damping[1]} step={1}
          value={damping} label="Damping" ariaLabel="Spring damping"
          formatValue={(v) => v.toFixed(0)}
          onChange={(v) => onChange(formatSpring(stiffness, v, mass))} />
      </label>
    </div>
  );
}
