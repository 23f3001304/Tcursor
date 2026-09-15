import { Picker, Segmented, Slider, Switch } from "../../controls/Controls";
import { CaptionColorFields } from "./CaptionColorFields";
import {
  ANIMATIONS,
  ANIM_MS_MAX,
  ANIM_MS_STEP,
  LINE_PCT_MAX,
  LINE_PCT_MIN,
  LINE_PCT_STEP,
  SIZE_RUNG_PCT,
  captionLinePct,
  captionRung,
  linePctText,
} from "./captionLook";
import type { CaptionPos, CaptionSize, CaptionStyle } from "../../../hud/settings/settings";

const POSITIONS: { value: CaptionPos; label: string }[] = [
  { value: "bottom", label: "Bottom" },
  { value: "top", label: "Top" },
];
const SIZES: { value: CaptionSize | ""; label: string }[] = [
  { value: "s", label: "Small" },
  { value: "m", label: "Medium" },
  { value: "l", label: "Large" },
];

export const DEFAULT_CAPTION_STYLE: CaptionStyle = {
  enabled: true,
  position: "bottom",
  size: "m",
  pill: true,
  highlight: true,
  model: "base.en",
  language: "en",
  font_pct: 0,
  text_color: [255, 255, 255],
  highlight_color: null,
  pill_color: [0, 0, 0],
  pill_alpha: 62,
  animation: "fade",
  animation_ms: 120,
};

export function CaptionStyleControls({
  style,
  accent,
  onChange,
}: {
  style: CaptionStyle;
  accent: [number, number, number];
  onChange: (s: CaptionStyle) => void;
}) {
  const set = <K extends keyof CaptionStyle>(k: K, v: CaptionStyle[K]) => onChange({ ...style, [k]: v });
  return (
    <div className="e-grp">
      <span className="e-sechead">Look</span>
      <div className="e-switchrow">
        <span>Show captions</span>
        <Switch on={style.enabled} onChange={(v) => set("enabled", v)} ariaLabel="Show captions" />
      </div>
      {style.enabled && (
        <>
          <div className="e-field">
            <span className="e-fl">Position</span>
            <Segmented
              value={style.position}
              options={POSITIONS}
              onChange={(v) => set("position", v)}
              ariaLabel="Position"
            />
          </div>
          <div className="e-field">
            <span className="e-fl">Size</span>
            <Segmented
              value={captionRung(style)}
              options={SIZES}
              onChange={(v) => v && onChange({ ...style, size: v, font_pct: SIZE_RUNG_PCT[v] })}
              ariaLabel="Size"
            />
          </div>
          <div className="e-field">
            <Slider
              value={captionLinePct(style)}
              min={LINE_PCT_MIN}
              max={LINE_PCT_MAX}
              step={LINE_PCT_STEP}
              onChange={(v) => set("font_pct", v)}
              label="Line height"
              ariaLabel="Line height"
              formatValue={linePctText}
            />
          </div>

          <CaptionColorFields style={style} accent={accent} onChange={onChange} />

          <div className="e-field">
            <span className="e-fl">Animation</span>
            <Picker
              value={style.animation}
              options={ANIMATIONS}
              onChange={(v) => set("animation", v)}
              ariaLabel="Animation"
            />
          </div>
          {style.animation !== "none" && (
            <div className="e-field">
              <Slider
                value={style.animation_ms}
                min={0}
                max={ANIM_MS_MAX}
                step={ANIM_MS_STEP}
                onChange={(v) => set("animation_ms", Math.round(v))}
                label="Duration"
                ariaLabel="Animation duration"
                formatValue={(v) => `${Math.round(v)} ms`}
              />
            </div>
          )}
        </>
      )}
    </div>
  );
}
