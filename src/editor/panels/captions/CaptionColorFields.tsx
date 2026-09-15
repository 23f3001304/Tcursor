import { ColorInput, Slider, Swatches, Switch, type SwatchItem } from "../../controls/Controls";
import { rgb, swatchItems } from "../effectSwatches";
import { HIGHLIGHT_COLORS, PILL_COLORS, TEXT_COLORS } from "./captionLook";
import type { CaptionStyle } from "../../../hud/settings/settings";

type Rgb = [number, number, number];

const same = (a: Rgb | null, b: Rgb | null) => (a && b ? rgb(a) === rgb(b) : a === b);

export function CaptionColorFields({
  style,
  accent,
  onChange,
}: {
  style: CaptionStyle;
  accent: Rgb;
  onChange: (s: CaptionStyle) => void;
}) {
  const set = <K extends keyof CaptionStyle>(k: K, v: CaptionStyle[K]) => onChange({ ...style, [k]: v });
  const highlights: SwatchItem<Rgb | null>[] = [
    { key: "accent", css: rgb(accent), value: null, ariaLabel: "Accent" },
    ...swatchItems(HIGHLIGHT_COLORS),
  ];
  return (
    <>
      <div className="e-field">
        <span className="e-fl">Text colour</span>
        <div className="e-caprow-colors">
          <Swatches
            items={swatchItems(TEXT_COLORS)}
            isSelected={(c) => rgb(c) === rgb(style.text_color)}
            onSelect={(c) => set("text_color", c)}
          />
          <ColorInput
            value={style.text_color}
            onChange={(c) => set("text_color", c)}
            ariaLabel="Pick any text colour"
          />
        </div>
      </div>

      <div className="e-switchrow">
        <span>Highlight the word being spoken</span>
        <Switch
          on={style.highlight}
          onChange={(v) => set("highlight", v)}
          ariaLabel="Highlight the word being spoken"
        />
      </div>
      {style.highlight && (
        <div className="e-field">
          <span className="e-fl">Highlight colour</span>
          <div className="e-caprow-colors">
            <Swatches
              items={highlights}
              isSelected={(c) => same(c, style.highlight_color)}
              onSelect={(c) => set("highlight_color", c)}
            />
            <ColorInput
              value={style.highlight_color ?? accent}
              onChange={(c) => set("highlight_color", c)}
              ariaLabel="Pick any highlight colour"
            />
          </div>
        </div>
      )}

      <div className="e-switchrow">
        <span>Background pill</span>
        <Switch on={style.pill} onChange={(v) => set("pill", v)} ariaLabel="Background pill" />
      </div>
      {style.pill && (
        <>
          <div className="e-field">
            <span className="e-fl">Pill colour</span>
            <div className="e-caprow-colors">
              <Swatches
                items={swatchItems(PILL_COLORS)}
                isSelected={(c) => rgb(c) === rgb(style.pill_color)}
                onSelect={(c) => set("pill_color", c)}
              />
              <ColorInput
                value={style.pill_color}
                onChange={(c) => set("pill_color", c)}
                ariaLabel="Pick any pill colour"
              />
            </div>
          </div>
          <div className="e-field">
            <Slider
              value={style.pill_alpha}
              min={0}
              max={100}
              step={1}
              onChange={(v) => set("pill_alpha", Math.round(v))}
              label="Opacity"
              ariaLabel="Pill opacity"
              formatValue={(v) => `${Math.round(v)}%`}
            />
          </div>
        </>
      )}
    </>
  );
}
