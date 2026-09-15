import { IconRotate2 } from "@tabler/icons-react";
import type { ZoomSettings } from "../../../hud/settings/settings";
import { Switch, Slider, NumberField, Picker } from "../../controls/Controls";

export const DEFAULT_ZOOM_SETTINGS: ZoomSettings = {
  enabled: true,
  target_scale: 2.2,
  hold_ms: 2200,
  smoothness: 0.1,
  clicks: 1,
  camera_shrink: true,
  camera_shrink_min: 0.62,
  smart_hold: true,
  smart_follow: false,
  cam_zoom_default: null,
  camera_smoothing_ms: 0,
};

const CLICKS_OPTS: { value: "1" | "2" | "3"; label: string }[] = [
  { value: "1", label: "1" },
  { value: "2", label: "2" },
  { value: "3", label: "3" },
];

export const clicksToOption = (clicks: number): "1" | "2" | "3" =>
  clicks <= 1 ? "1" : clicks === 2 ? "2" : "3";
export const clicksFromOption = (opt: string): number => Number(opt);

export function ZoomDefaultsSection({
  value,
  onChange,
}: {
  value: ZoomSettings;
  onChange: (v: ZoomSettings) => void;
}) {
  const set = <K extends keyof ZoomSettings>(k: K, v: ZoomSettings[K]) => onChange({ ...value, [k]: v });
  const holdLabel = value.smart_hold ? "Idle release" : "Hold";
  const holdSec = +(value.hold_ms / 1000).toFixed(1);

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Zoom defaults</span>
        <button
          type="button"
          className="e-hicon"
          title="Reset to defaults"
          aria-label="Reset zoom defaults"
          onClick={() => onChange(DEFAULT_ZOOM_SETTINGS)}
        >
          <IconRotate2 size={14} />
        </button>
      </div>
      <p className="e-lede" style={{ margin: "4px 0 14px" }}>
        Defaults for new and automatic zooms. Existing zoom regions keep their own values.
      </p>

      <div className="e-field">
        <div className="e-switchrow">
          <span>Zoom on click</span>
          <Switch on={value.enabled} onChange={(v) => set("enabled", v)} />
        </div>
      </div>

      <div className="e-field">
        <Slider
          min={1.2}
          max={4}
          step={0.1}
          value={value.target_scale}
          onChange={(v) => set("target_scale", v)}
          ariaLabel="Zoom amount"
          label="Zoom amount"
          formatValue={(v) => `${v.toFixed(1)}x`}
        />
      </div>

      <div className="e-field">
        <span className="e-fl">
          {holdLabel} <b>{holdSec}s</b>
        </span>
        <NumberField
          min={0.6}
          max={5}
          step={0.1}
          unit="s"
          value={holdSec}
          onChange={(v) => set("hold_ms", Math.round(v * 1000))}
        />
      </div>

      <div className="e-field">
        <Slider
          min={0.04}
          max={0.3}
          step={0.01}
          value={value.smoothness}
          onChange={(v) => set("smoothness", v)}
          ariaLabel="Smoothness"
          label="Smoothness"
          formatValue={(v) => v.toFixed(2)}
        />
      </div>

      <div className="e-field">
        <span className="e-fl">Clicks to zoom</span>
        <Picker
          value={clicksToOption(value.clicks)}
          options={CLICKS_OPTS}
          onChange={(opt) => set("clicks", clicksFromOption(opt))}
          ariaLabel="Clicks to zoom"
        />
      </div>

      <div className="e-field">
        <div className="e-switchrow">
          <span>Shrink camera on zoom</span>
          <Switch on={value.camera_shrink} onChange={(v) => set("camera_shrink", v)} />
        </div>
      </div>

      {value.camera_shrink && (
        <div className="e-field">
          <Slider
            min={0.3}
            max={1}
            step={0.02}
            value={value.camera_shrink_min}
            onChange={(v) => set("camera_shrink_min", v)}
            ariaLabel="Min camera size"
            label="Min camera size"
            formatValue={(v) => `${Math.round(v * 100)}%`}
          />
        </div>
      )}

      <div className="e-field">
        <div className="e-switchrow">
          <span>Smart type</span>
          <Switch on={value.smart_hold} onChange={(v) => set("smart_hold", v)} />
        </div>
      </div>

      <div className="e-field" style={{ marginBottom: 0 }}>
        <div className="e-switchrow">
          <span>Smart follow</span>
          <Switch on={value.smart_follow} onChange={(v) => set("smart_follow", v)} />
        </div>
      </div>
    </div>
  );
}
