import type { GradePreset, GradeSettings } from "../../../hud/settings/settings";
import { Picker, Slider } from "../../controls/Controls";
import { seedOf } from "../../stage/grade/gradeParams";

const LOOKS: { value: GradePreset; label: string }[] = [
  { value: "none", label: "None" },
  { value: "cinematic", label: "Cinematic" },
  { value: "noir", label: "Noir" },
  { value: "vintage", label: "Vintage" },
  { value: "frost", label: "Frost" },
  { value: "golden", label: "Golden" },
  { value: "midnight", label: "Midnight" },
  { value: "vivid", label: "Vivid" },
  { value: "dreamy", label: "Dreamy" },
];

export const NO_GRADE: GradeSettings = { preset: "none", exposure: 0, contrast: 1, vignette: 0 };

const EPS = 1e-6;

export function hasDrifted(g: GradeSettings): boolean {
  if (g.preset === "none") return false;
  const [exposure, contrast, vignette] = seedOf(g.preset);
  return (
    Math.abs(g.exposure - exposure) > EPS ||
    Math.abs(g.contrast - contrast) > EPS ||
    Math.abs(g.vignette - vignette) > EPS
  );
}

export function GradeSection({
  grade,
  onChange,
}: {
  grade: GradeSettings;
  onChange: (next: GradeSettings) => void;
}) {
  const pick = (preset: GradePreset) => {
    const [exposure, contrast, vignette] = seedOf(preset);
    onChange({ preset, exposure, contrast, vignette });
  };
  const set = (patch: Partial<GradeSettings>) => onChange({ ...grade, ...patch });
  return (
    <div className="e-grp">
      <span className="e-sechead">Color</span>
      <div className="e-field">
        <span className="e-fl">Look</span>
        <Picker
          value={grade.preset}
          options={LOOKS}
          onChange={pick}
          ariaLabel="Look"
          label={hasDrifted(grade) ? "Custom" : undefined}
        />
      </div>
      <div className="e-field">
        <Slider
          min={-2}
          max={2}
          step={0.05}
          value={grade.exposure}
          onChange={(exposure) => set({ exposure })}
          ariaLabel="Exposure"
          label="Exposure"
          formatValue={(v) => `${v >= 0 ? "+" : ""}${v.toFixed(2)} EV`}
        />
      </div>
      <div className="e-field">
        <Slider
          min={0.5}
          max={1.8}
          step={0.02}
          value={grade.contrast}
          onChange={(contrast) => set({ contrast })}
          ariaLabel="Contrast"
          label="Contrast"
          formatValue={(v) => `${Math.round(v * 100)}%`}
        />
      </div>
      <div className="e-field">
        <Slider
          min={0}
          max={100}
          step={1}
          value={Math.round(grade.vignette * 100)}
          onChange={(v) => set({ vignette: v / 100 })}
          ariaLabel="Vignette"
          label="Vignette"
          formatValue={(v) => `${Math.round(v)}%`}
        />
      </div>
      <span className="e-hintline">
        The look applies to the whole picture. Captions, text and the cursor keep their own colours.
      </span>
    </div>
  );
}
