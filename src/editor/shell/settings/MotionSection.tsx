import { IconRotate2 } from "@tabler/icons-react";
import type { MotionSettings } from "../../../hud/settings/settings";
import { Picker, Slider } from "../../controls/Controls";
import { PRESETS, presetOf, presetPatch } from "../../motion/presets";
import type { GraphInput } from "../../motion/graphModel";
import { MotionGraph } from "../../motion/MotionGraph";

export const defaultGraphInput = (m: MotionSettings): GraphInput => ({
  lane: "zoom",
  startMs: 0,
  endMs: 3000,
  peak: 2,
  rampIn: { easing: m.easing, durMs: 450 },
  rampOut: { easing: m.easing_out, durMs: 700 },
});

export const DEFAULT_MOTION_RESET: MotionSettings = {
  preset: "soft",
  easing: "smooth",
  easing_out: "smooth",
};

const PRESET_OPTS = PRESETS.map((p) => ({ value: p.id as string, label: p.name, title: p.feel }));

const CUSTOM_OPT = { value: "custom", label: "Custom", title: "These curves match no preset" };

export const smoothingLagMs = (ms: number): number => Math.round(ms * 0.3);

export function MotionSection({
  value,
  onChange,
  smoothingMs,
  onSmoothingChange,
  onApplyToAll,
}: {
  value: MotionSettings;
  onChange: (v: MotionSettings) => void;
  smoothingMs: number;
  onSmoothingChange: (ms: number) => void;
  onApplyToAll: () => void;
}) {
  const current = presetOf(value.easing, value.easing_out);
  const options = current === "custom" ? [...PRESET_OPTS, CUSTOM_OPT] : PRESET_OPTS;
  const feel = PRESETS.find((p) => p.id === current)?.feel ?? CUSTOM_OPT.title;
  const pick = (id: string) => onChange({ preset: id, ...presetPatch(id) });
  const lag = smoothingLagMs(smoothingMs);

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Motion</span>
        <button
          type="button"
          className="e-hicon"
          title="Reset to default"
          aria-label="Reset motion settings"
          onClick={() => onChange(DEFAULT_MOTION_RESET)}
        >
          <IconRotate2 size={14} />
        </button>
      </div>
      <p className="e-lede" style={{ margin: "4px 0 14px" }}>
        One feel for the whole project. New zooms, layouts and camera moves start here.
      </p>

      <div className="e-field">
        <span className="e-fl">Feel</span>
        <Picker value={current} options={options} onChange={pick} ariaLabel="Motion preset" />
        <span className="e-lede" style={{ marginTop: 4 }}>
          {feel}
        </span>
      </div>

      <div className="e-field">
        <MotionGraph readOnly input={defaultGraphInput(value)} onCommit={() => {}} />
      </div>

      <div className="e-field">
        <button type="button" className="e-modal-btn" onClick={onApplyToAll}>
          Apply to all regions
        </button>
        <span className="e-lede" style={{ marginTop: 4 }}>
          Puts this feel on every zoom, layout and camera move already on the timeline. One undo step.
        </span>
      </div>

      <div className="e-field" style={{ marginBottom: 0 }}>
        <Slider
          min={0}
          max={400}
          step={10}
          value={smoothingMs}
          onChange={(v) => onSmoothingChange(Math.round(v))}
          ariaLabel="Camera smoothing"
          label="Camera smoothing"
          formatValue={(v) => (v === 0 ? "Off" : `${v} ms, ${smoothingLagMs(v)} ms lag`)}
        />
        <span className="e-lede" style={{ marginTop: 4 }}>
          {smoothingMs === 0
            ? "Smooths the auto-zoom camera's path. Higher is calmer but lags the cursor - 120 ms is a good start."
            : `Smooths the auto-zoom camera's path. This setting costs about ${lag} ms of lag behind the cursor.`}
        </span>
      </div>
    </div>
  );
}
