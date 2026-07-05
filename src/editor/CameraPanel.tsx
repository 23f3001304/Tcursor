import { useState } from "react";
import { IconX } from "@tabler/icons-react";
import type { AppearanceSettings, ModeAppearance } from "../hud/settings";
import { Slider, Picker } from "./Controls";
import {
  MODES,
  MODE_SLIDERS,
  MODE_HAS_SHAPE,
  MODE_HAS_CORNER,
  SLIDERS,
  SHAPES,
  CORNERS,
  pct,
  DEFAULT_APPEARANCE,
  type ModeKey,
} from "../hud/appearanceFields";

export function CameraPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: AppearanceSettings;
  onChange: (v: AppearanceSettings) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<ModeKey>("screen");
  const ma = settings[mode] || DEFAULT_APPEARANCE[mode];

  const set = <K extends keyof ModeAppearance>(k: K, v: ModeAppearance[K]) => {
    onChange({
      ...settings,
      [mode]: { ...ma, [k]: v },
    });
  };

  return (
    <div className="e-panel e-insp">
      <div className="e-insphdr">
        <h2>Camera & PIP</h2>
        <button className="e-gst" title="Close" onClick={onClose}><IconX size={16} /></button>
      </div>
      <p className="e-lede">Customize size, shape layouts, and corners for picture-in-picture frames.</p>

      {/* Select Layout Mode */}
      <div className="e-field">
        <span className="e-fl">Select Layout Mode</span>
        <Picker
          value={mode}
          options={MODES.map(([id, label]) => ({ value: id, label }))}
          onChange={setMode}
        />
      </div>

      {/* Mode Sliders */}
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {MODE_SLIDERS[mode].map((k) => {
          const spec = SLIDERS[k];
          return (
            <div className="e-field" key={k} style={{ marginBottom: 0 }}>
              <span className="e-fl">{spec.label} <b>{pct(ma[k])}</b></span>
              <Slider
                min={spec.min}
                max={spec.max}
                step={spec.step}
                value={ma[k]}
                onChange={(v) => set(k, v)}
              />
            </div>
          );
        })}
      </div>

      {/* Webcam Shape */}
      {MODE_HAS_SHAPE[mode] && (
        <div className="e-field" style={{ marginTop: 12 }}>
          <span className="e-fl">Webcam Shape</span>
          <div className="e-seg">
            {SHAPES.map(([shapeId, label]) => (
              <button
                key={shapeId}
                type="button"
                className={ma.cam_shape === shapeId ? "on" : ""}
                onClick={() => set("cam_shape", shapeId)}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Corner radius for rounded shapes */}
      {MODE_HAS_SHAPE[mode] && ma.cam_shape === "rounded" && (
        <div className="e-field">
          <span className="e-fl">Corner Roundness <b>{pct(ma.cam_radius)}</b></span>
          <Slider
            min={SLIDERS.cam_radius.min}
            max={SLIDERS.cam_radius.max}
            step={SLIDERS.cam_radius.step}
            value={ma.cam_radius}
            onChange={(v) => set("cam_radius", v)}
          />
        </div>
      )}

      {/* Corner dock margins */}
      {MODE_HAS_CORNER[mode] && (
        <div className="e-field">
          <span className="e-fl">Dock Location</span>
          <Picker
            value={ma.cam_corner}
            options={CORNERS.map(([id, label]) => ({ value: id, label: label.replace("_", " ") }))}
            onChange={(v) => set("cam_corner", v)}
          />
        </div>
      )}

      <button
        type="button"
        className="e-del"
        style={{ marginTop: 16 }}
        onClick={() => onChange(DEFAULT_APPEARANCE)}
      >
        Reset Defaults
      </button>
    </div>
  );
}
