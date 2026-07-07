import type { CamRing } from "../../hud/settings/settings";
import { Switch, Slider } from "../controls/Controls";
import { RING_WIDTH_SLIDER, DEFAULT_RING, pct } from "../../hud/preferences/appearanceFields";

const SWATCHES: [number, number, number][] = [
  [255, 255, 255], [239, 68, 68], [59, 130, 246], [34, 197, 94], [245, 158, 11]
];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

/** Webcam ring/border controls: on/off switch (null <-> DEFAULT_RING), then (when on) a
 *  width slider and a color swatch row - extracted from CameraPanel so that file stays
 *  under the line limit. Mirrors EffectsPanel's ripple-color swatch pattern for consistency. */
export function CameraRingField({ ring, onChange }: { ring: CamRing | null; onChange: (v: CamRing | null) => void }) {
  return (
    <>
      <div className="e-field" style={{ marginTop: 12 }}>
        <span className="e-fl">Ring</span>
        <Switch on={ring !== null} onChange={(v) => onChange(v ? DEFAULT_RING : null)} />
      </div>
      {ring && (
        <>
          <div className="e-field">
            <span className="e-fl">Ring Width <b>{pct(ring.width)}</b></span>
            <Slider
              min={RING_WIDTH_SLIDER.min}
              max={RING_WIDTH_SLIDER.max}
              step={RING_WIDTH_SLIDER.step}
              value={ring.width}
              onChange={(v) => onChange({ ...ring, width: v })}
            />
          </div>
          <div className="e-field">
            <span className="e-fl">Ring Color</span>
            <div style={{ display: "flex", gap: 8, marginTop: 2 }}>
              {SWATCHES.map((c) => {
                const colorStr = rgb(c);
                const isSelected = rgb(ring.color) === colorStr;
                return (
                  <button
                    key={colorStr}
                    type="button"
                    style={{
                      background: colorStr,
                      border: isSelected ? "2px solid var(--e-fg)" : "1px solid var(--e-border)",
                      width: 22,
                      height: 22,
                      borderRadius: "50%",
                      cursor: "pointer",
                      padding: 0,
                      outline: "none"
                    }}
                    onClick={() => onChange({ ...ring, color: c })}
                  />
                );
              })}
            </div>
          </div>
        </>
      )}
    </>
  );
}
