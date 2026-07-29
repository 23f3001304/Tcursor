import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { IconFolderPlus } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import type { CursorSettings, CursorStyle } from "../../hud/settings/settings";
import type { CursorPackInfo } from "../../lib/ipc";
import { listCursorPacks, importCursorPack } from "../../lib/ipc";
import { Switch, Slider, Picker } from "../controls/Controls";

// System keeps the OS cursor already baked into the recording; Enhanced redraws a smooth synthetic
// pointer (from the recorded cursor path, captured for every style); Hidden shows none. Having all
// three here - not just Enhanced/Hidden - is what lets a clip recorded in System go back to its
// original cursor in the editor.
const STYLE_OPTS: { value: CursorStyle; label: string }[] = [
  { value: "system", label: "System — original cursor" },
  { value: "enhanced", label: "Enhanced — redrawn" },
  { value: "hidden", label: "Hidden" },
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
  const [packs, setPacks] = useState<CursorPackInfo[]>([]);
  const [importing, setImporting] = useState(false);
  const [importErr, setImportErr] = useState("");

  useEffect(() => { listCursorPacks().then(setPacks).catch(() => {}); }, []);

  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  const handleReset = () => {
    onChange({
      style: "enhanced",
      size: 1.0,
      smoothness: 0.6,
      path_idealize: 0.0,
      motion_blur: 0.35,
      click_bounce: true,
      bounce_intensity: 0.5,
      pack: "default",
    });
  };

  const handleImport = async () => {
    setImportErr("");
    const dir = await open({ directory: true, multiple: false, title: "Choose a cursor pack folder" }).catch(() => null);
    if (!dir || Array.isArray(dir)) return;
    setImporting(true);
    try {
      const info = await importCursorPack(dir);
      setPacks((prev) => [...prev, info]);
      set("pack", info.id);
    } catch (e) {
      setImportErr(typeof e === "string" ? e : "Import failed - check the folder has cursor PNGs.");
    } finally {
      setImporting(false);
    }
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Cursor" lede="Cursor shape, size, and motion-trail feel."
        onReset={handleReset} onClose={onClose} />

      {/* Style: System (keep the recorded OS cursor) / Enhanced (redraw) / Hidden. The pack, size,
          and motion controls below only apply to Enhanced, so they're hidden for the other two. */}
      <div className="e-field">
        <span className="e-fl">Cursor style</span>
        <Picker value={settings.style} options={STYLE_OPTS} onChange={(style) => set("style", style)} />
      </div>

      {settings.style === "enhanced" && (
        <>
          <div className="e-field">
            <div className="e-switchrow">
              <span>Click bounce animation</span>
              <Switch on={settings.click_bounce} onChange={(v) => set("click_bounce", v)} />
            </div>
          </div>

          {/* Cursor Packs */}
          <div className="e-field">
            <span className="e-sechead">Cursor style pack</span>
            <div className="e-pack-grid">
              {packs.map((p) => {
                const isSelected = settings.pack === p.id;
                return (
                  <button
                    key={p.id}
                    type="button"
                    className={`e-pack-card ${isSelected ? "on" : ""}`}
                    title={p.builtin ? `${p.name} (built-in)` : p.name}
                    onClick={() => set("pack", p.id)}
                  >
                    <span>{p.name}</span>
                  </button>
                );
              })}
            </div>
            <button type="button" className="e-upload-dashed" onClick={handleImport} disabled={importing}>
              <IconFolderPlus size={14} /> {importing ? "Importing..." : "Import pack..."}
            </button>
            {importErr && <span className="e-fl" style={{ color: "#f87171" }}>{importErr}</span>}
          </div>

          {/* Sliders */}
          <div className="e-field">
            <span className="e-fl">Cursor Size <b>{settings.size.toFixed(2)}x</b></span>
            <Slider min={0.4} max={3.0} step={0.1} value={settings.size} onChange={(v) => set("size", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Cursor Smoothness <b>{settings.smoothness.toFixed(2)}</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.smoothness} onChange={(v) => set("smoothness", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Path Idealization <b>{settings.path_idealize.toFixed(2)}</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.path_idealize} onChange={(v) => set("path_idealize", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Motion Trail Blur <b>{settings.motion_blur.toFixed(2)}x</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.motion_blur} onChange={(v) => set("motion_blur", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Click Bounce Intensity <b>{settings.bounce_intensity.toFixed(2)}x</b></span>
            <Slider min={0.1} max={1.0} step={0.05} value={settings.bounce_intensity} onChange={(v) => set("bounce_intensity", v)} />
          </div>
        </>
      )}
    </div>
  );
}
