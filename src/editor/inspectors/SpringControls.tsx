import { Slider } from "../controls/fields/Slider";
import { formatSpring, SPRING_RANGE } from "../../shared/math/spring";

export function SpringControls({
  stiffness,
  damping,
  mass,
  onChange,
}: {
  stiffness: number;
  damping: number;
  mass: number;
  onChange: (easing: string) => void;
}) {
  return (
    <div className="e-field2">
      <label className="e-field">
        <Slider
          min={SPRING_RANGE.stiffness[0]}
          max={SPRING_RANGE.stiffness[1]}
          step={1}
          value={stiffness}
          label="Stiffness"
          ariaLabel="Spring stiffness"
          formatValue={(v) => v.toFixed(0)}
          onChange={(v) => onChange(formatSpring(v, damping, mass))}
        />
      </label>
      <label className="e-field">
        <Slider
          min={SPRING_RANGE.damping[0]}
          max={SPRING_RANGE.damping[1]}
          step={1}
          value={damping}
          label="Damping"
          ariaLabel="Spring damping"
          formatValue={(v) => v.toFixed(0)}
          onChange={(v) => onChange(formatSpring(stiffness, v, mass))}
        />
      </label>
    </div>
  );
}
