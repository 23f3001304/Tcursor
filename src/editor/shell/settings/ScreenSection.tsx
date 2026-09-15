import { IconRotate2 } from "@tabler/icons-react";
import type { ModeAppearance } from "../../../hud/settings/settings";
import { Slider } from "../../controls/Controls";

export const DEFAULT_SCREEN_SIZE = 1.0;

export function ScreenSection({
  value,
  onChange,
}: {
  value: ModeAppearance;
  onChange: (v: ModeAppearance) => void;
}) {
  const setSize = (screen_size: number) => onChange({ ...value, screen_size });

  return (
    <div className="e-sec">
      <div className="e-secrow">
        <span className="e-sechead">Screen</span>
        <button
          type="button"
          className="e-hicon"
          title="Reset to default"
          aria-label="Reset screen size"
          onClick={() => setSize(DEFAULT_SCREEN_SIZE)}
        >
          <IconRotate2 size={14} />
        </button>
      </div>
      <div className="e-field" style={{ marginBottom: 0 }}>
        <Slider
          min={0.6}
          max={1}
          step={0.01}
          value={value.screen_size}
          onChange={setSize}
          ariaLabel="Screen size"
          label="Screen size"
          formatValue={(v) => `${Math.round(v * 100)}%`}
        />
      </div>
    </div>
  );
}
