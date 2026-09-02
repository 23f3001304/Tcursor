import type { CamRing } from "../../hud/settings/settings";
import { Switch, Slider, Swatches, type SwatchItem } from "../controls/Controls";
import { RING_WIDTH_SLIDER, DEFAULT_RING, pct } from "../../hud/preferences/appearanceFields";

// [color, human name] - the name becomes each swatch's aria-label.
const SWATCHES: [[number, number, number], string][] = [
  [[255, 255, 255], "White"], [[239, 68, 68], "Red"], [[59, 130, 246], "Blue"], [[34, 197, 94], "Green"], [[245, 158, 11], "Orange"],
];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const swatchItems: SwatchItem<[number, number, number]>[] = SWATCHES.map(([c, name]) => ({ key: rgb(c), css: rgb(c), value: c, ariaLabel: name }));

/** Webcam ring/border controls: on/off switch (null <-> DEFAULT_RING), then (when on) a
 *  width slider and a color swatch row - extracted from CameraPanel so that file stays
 *  under the line limit. Uses the shared `Swatches` component (Task 26), same as
 *  EffectsPanel's ripple-color/tint rows and BackgroundPanel's presets. */
export function CameraRingField({ ring, onChange }: { ring: CamRing | null; onChange: (v: CamRing | null) => void }) {
  return (
    <>
      <div className="e-field" style={{ marginTop: 16 }}>
        <div className="e-switchrow">
          <span>Ring</span>
          <Switch on={ring !== null} onChange={(v) => onChange(v ? DEFAULT_RING : null)} />
        </div>
      </div>
      {ring && (
        <>
          <div className="e-field">
            <Slider
              min={RING_WIDTH_SLIDER.min}
              max={RING_WIDTH_SLIDER.max}
              step={RING_WIDTH_SLIDER.step}
              value={ring.width}
              onChange={(v) => onChange({ ...ring, width: v })}
              ariaLabel="Ring Width"
              label="Ring Width"
              formatValue={pct}
            />
          </div>
          <div className="e-field">
            <span className="e-fl">Ring Color</span>
            <Swatches items={swatchItems} isSelected={(c) => rgb(c) === rgb(ring.color)}
              onSelect={(c) => onChange({ ...ring, color: c })} />
          </div>
        </>
      )}
    </>
  );
}
