import { useState } from "react";
import { IconX, IconPlus } from "@tabler/icons-react";
import type { CursorSettings } from "../hud/settings";
import { Switch, Slider } from "./Controls";

const PACKS = [
  { id: "mac", path: "M 0 0 L 0 15 L 4 11.5 L 9 16.5 L 11 14.5 L 6 9.5 L 11 9 Z", label: "macOS" },
  { id: "win", path: "M 3 0 L 15 12 L 9 12 L 13 18 L 10 19.5 L 6.5 13.5 L 3 17 Z", label: "Windows" },
  { id: "white_circle", path: "M 6 0 A 6 6 0 1 0 6 12 A 6 6 0 1 0 6 0 Z", label: "Dot" },
  { id: "black_pointer", path: "M 0 0 L 12 12 L 5 12 L 0 7 Z", label: "Classic" },
];

export function CursorPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: CursorSettings;
  onChange: (v: CursorSettings) => void;
  onClose: () => void;
}) {
  const [showCursor, setShowCursor] = useState(true);
  const [loopCursor, setLoopCursor] = useState(false);
  const [selectedPack, setSelectedPack] = useState("mac");

  // Local sliders state loaded from localStorage if present
  const [smoothing, setSmoothing] = useState(() => {
    const v = localStorage.getItem("tcursor_smoothing");
    return v ? parseFloat(v) : 0.67;
  });
  const [sway, setSway] = useState(() => {
    const v = localStorage.getItem("tcursor_sway");
    return v ? parseFloat(v) : 0.13;
  });
  const [bounceSpeed, setBounceSpeed] = useState(() => {
    const v = localStorage.getItem("tcursor_bounce_speed");
    return v ? parseInt(v, 10) : 350;
  });
  const [bounceInt, setBounceInt] = useState(() => {
    const v = localStorage.getItem("tcursor_bounce_int");
    return v ? parseFloat(v) : 3.50;
  });

  const changeSmoothing = (v: number) => { setSmoothing(v); localStorage.setItem("tcursor_smoothing", v.toString()); };
  const changeSway = (v: number) => { setSway(v); localStorage.setItem("tcursor_sway", v.toString()); };
  const changeBounceSpeed = (v: number) => { setBounceSpeed(v); localStorage.setItem("tcursor_bounce_speed", v.toString()); };
  const changeBounceInt = (v: number) => { setBounceInt(v); localStorage.setItem("tcursor_bounce_int", v.toString()); };

  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  const handleReset = () => {
    setShowCursor(true);
    setLoopCursor(false);
    setSelectedPack("mac");
    changeSmoothing(0.67);
    changeSway(0.13);
    changeBounceSpeed(350);
    changeBounceInt(3.50);
    set("size", 2.2);
    set("motion_blur", 0.4);
  };

  return (
    <div className="e-panel e-insp">
      <div className="e-insphdr">
        <h2>Cursor Settings</h2>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <button
            type="button"
            onClick={handleReset}
            style={{
              border: "none",
              background: "transparent",
              color: "var(--accent, #ef4444)",
              fontSize: 12.5,
              fontWeight: 550,
              padding: "4px 8px",
              cursor: "pointer"
            }}
          >
            Reset
          </button>
          <button className="e-gst" title="Close" onClick={onClose}><IconX size={16} /></button>
        </div>
      </div>
      <p className="e-lede">Customize cursor shape styles, size scaling, and motion trail paths.</p>

      {/* Switches in Grid */}
      <div className="e-field">
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <span style={{ fontSize: 13, color: "var(--e-fg)" }}>Show cursor</span>
          <Switch on={showCursor} onChange={setShowCursor} />
        </div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span style={{ fontSize: 13, color: "var(--e-fg)" }}>Loop cursor motion</span>
          <Switch on={loopCursor} onChange={setLoopCursor} />
        </div>
      </div>

      {/* Cursor Packs */}
      <div className="e-field">
        <span className="e-fl">Cursor style pack</span>
        <div className="e-pack-grid">
          {PACKS.map((p) => {
            const isSelected = selectedPack === p.id;
            return (
              <button
                key={p.id}
                type="button"
                className={`e-pack-card ${isSelected ? "on" : ""}`}
                onClick={() => setSelectedPack(p.id)}
              >
                <svg viewBox="0 0 20 20" style={{ width: 14, height: 14, fill: "var(--e-fg)", stroke: "var(--e-bg)", strokeWidth: 1, pointerEvents: "none" }}>
                  <path d={p.path} />
                </svg>
                <span>{p.label}</span>
              </button>
            );
          })}
          
          {/* Add custom pack */}
          <button
            type="button"
            onClick={() => alert("Upload custom cursor pack (.zip)")}
            className="e-pack-add"
          >
            <IconPlus size={14} style={{ pointerEvents: "none" }} />
            <span>Add</span>
          </button>
        </div>
      </div>

      {/* Sliders */}
      <div className="e-field">
        <span className="e-fl">Cursor Size <b>{settings.size.toFixed(2)}x</b></span>
        <Slider min={0.5} max={4.0} step={0.1} value={settings.size} onChange={(v) => set("size", v)} />
      </div>

      <div className="e-field">
        <span className="e-fl">Smoothing <b>{smoothing.toFixed(2)}</b></span>
        <Slider min={0.0} max={1.0} step={0.01} value={smoothing} onChange={changeSmoothing} />
      </div>

      <div className="e-field">
        <span className="e-fl">Motion Trail Blur <b>{settings.motion_blur.toFixed(2)}x</b></span>
        <Slider min={0.0} max={1.0} step={0.05} value={settings.motion_blur} onChange={(v) => set("motion_blur", v)} />
      </div>

      <div className="e-field">
        <span className="e-fl">Click Bounce <b>{bounceInt.toFixed(2)}x</b></span>
        <Slider min={1.0} max={5.0} step={0.05} value={bounceInt} onChange={changeBounceInt} />
      </div>

      <div className="e-field">
        <span className="e-fl">Bounce Duration <b>{bounceSpeed} ms</b></span>
        <Slider min={100} max={1000} step={10} value={bounceSpeed} onChange={changeBounceSpeed} />
      </div>

      <div className="e-field">
        <span className="e-fl">Cursor Sway <b>{sway.toFixed(2)}x</b></span>
        <Slider min={0.0} max={1.0} step={0.01} value={sway} onChange={changeSway} />
      </div>
    </div>
  );
}
