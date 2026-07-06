import { useState } from "react";
import { IconUpload } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import type { InterfaceSettings } from "../../hud/settings/settings";
import { Switch, Slider } from "../controls/Controls";
import { COLOR_PRESETS, GRADIENT_PRESETS, IMAGE_PRESETS, VIDEO_PRESETS, ACCENTS } from "./backgroundPresets";

type BgType = "image" | "video" | "color" | "gradient";
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

export function BackgroundPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: InterfaceSettings;
  onChange: (v: InterfaceSettings) => void;
  onClose: () => void;
}) {
  const [blur, setBlur] = useState(0.0);
  const [bgType, setBgType] = useState<BgType>("gradient");
  const [selectedPreset, setSelectedPreset] = useState(0);
  const [shadow, setShadow] = useState(24);
  const [radius, setRadius] = useState(43);
  const [padding, setPadding] = useState(20);
  const [removeBg, setRemoveBg] = useState(false);

  const handleReset = () => {
    setBlur(0);
    setBgType("gradient");
    setShadow(24);
    setRadius(43);
    setPadding(20);
    setRemoveBg(false);
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Background" lede="Frame padding, shadow, corner, and background style."
        onReset={handleReset} onClose={onClose} />

      {/* Background Type Selector */}
      <div className="e-field">
        <span className="e-sechead">Background Type</span>
        <div className="e-seg">
          {(["image", "video", "color", "gradient"] as BgType[]).map((t) => (
            <button
              key={t}
              type="button"
              className={bgType === t ? "on" : ""}
              onClick={() => setBgType(t)}
              style={{ textTransform: "capitalize" }}
            >
              {t}
            </button>
          ))}
        </div>
      </div>

      {/* Upload Button */}
      <div className="e-field">
        <button className="e-upload-dashed" onClick={() => alert("Upload custom background image")}>
          <IconUpload size={14} /> Upload Custom
        </button>
      </div>

      {/* Presets Grid */}
      <div className="e-field">
        <span className="e-sechead">Presets</span>
        <div className="e-preset-grid">
          {(() => {
            const activePresets =
              bgType === "color"
                ? COLOR_PRESETS
                : bgType === "image"
                ? IMAGE_PRESETS
                : bgType === "video"
                ? VIDEO_PRESETS
                : GRADIENT_PRESETS;
            return activePresets.map((preset, idx) => {
              const isSelected = selectedPreset === idx;
              return (
                <button
                  key={idx}
                  type="button"
                  className={`e-preset-circle ${isSelected ? "on" : ""}`}
                  style={{
                    background: preset,
                    backgroundSize: "cover",
                    backgroundPosition: "center"
                  }}
                  onClick={() => setSelectedPreset(idx)}
                />
              );
            });
          })()}
        </div>
      </div>

      {/* Accent Colors */}
      <div className="e-field">
        <span className="e-sechead">Accent Colors</span>
        <div className="e-accent-list">
          {ACCENTS.map((c) => {
            const colorStr = rgb(c);
            const isSelected = rgb(settings.accent) === colorStr;
            return (
              <button
                key={colorStr}
                type="button"
                className={`e-accent-circle ${isSelected ? "on" : ""}`}
                style={{ background: colorStr }}
                onClick={() => onChange({ ...settings, accent: c })}
              />
            );
          })}
        </div>
      </div>

      {/* Custom Sliders */}
      <div className="e-field">
        <span className="e-fl">Background Blur <b>{blur.toFixed(1)}px</b></span>
        <Slider min={0} max={20} step={0.5} value={blur} onChange={setBlur} />
      </div>

      <div className="e-field">
        <span className="e-fl">Frame Shadow <b>{shadow}%</b></span>
        <Slider min={0} max={100} step={1} value={shadow} onChange={setShadow} />
      </div>

      <div className="e-field">
        <span className="e-fl">Corner Radius <b>{radius}px</b></span>
        <Slider min={0} max={80} step={1} value={radius} onChange={setRadius} />
      </div>

      <div className="e-field">
        <span className="e-fl">Padding <b>{padding}%</b></span>
        <Slider min={0} max={50} step={1} value={padding} onChange={setPadding} />
      </div>

      {/* Switches */}
      <div className="e-sec">
        <div className="e-switchrow">
          <span>Remove background</span>
          <Switch on={removeBg} onChange={setRemoveBg} />
        </div>
      </div>
    </div>
  );
}
