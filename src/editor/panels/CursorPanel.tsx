import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { IconFolderPlus } from "@tabler/icons-react";
import { PanelHeader } from "./PanelHeader";
import type { CursorSettings, CursorStyle } from "../../hud/settings/settings";
import type { CursorPackInfo } from "../../lib/ipc";
import { listCursorPacks, importCursorPack } from "../../lib/ipc";
import { Switch, Slider, Picker } from "../controls/Controls";
import { Shimmer } from "../timeline/Shimmer";

// Loading-skeleton tile count for the pack grid (`.e-pack-grid` is `repeat(5, 1fr)`, so 5 fills
// exactly one row - close enough to what a real pack list looks like without over-committing).
const PACK_SKELETON_COUNT = 5;

// System keeps the OS cursor already baked into the recording; Enhanced redraws a smooth synthetic
// pointer (from the recorded cursor path, captured for every style); Hidden shows none. Having all
// three here - not just Enhanced/Hidden - is what lets a clip recorded in System go back to its
// original cursor in the editor.
const STYLE_OPTS: { value: CursorStyle; label: string }[] = [
  { value: "system", label: "System — original cursor" },
  { value: "enhanced", label: "Enhanced — redrawn" },
  { value: "hidden", label: "Hidden" },
];

// Mirrors the Rust `CursorSettings::default()` (settings/model.rs) byte-for-byte, including
// `style: "system"` - a prior bug reset to `"enhanced"` here, silently diverging from the
// backend default every time a user pressed Reset.
export const DEFAULT_CURSOR_SETTINGS: CursorSettings = {
  style: "system",
  size: 1.0,
  smoothness: 0.6,
  path_idealize: 0.0,
  motion_blur: 0.35,
  click_bounce: true,
  bounce_intensity: 0.5,
  pack: "default",
};

export function CursorPanel({
  settings,
  onChange,
  onClose,
  // Optional so the panel still renders standalone (and in its own tests) as if the recording
  // did bake an OS cursor - i.e. no hint, today's wording.
  osCursorInVideo = true,
}: {
  settings: CursorSettings;
  onChange: (v: CursorSettings) => void;
  onClose: () => void;
  osCursorInVideo?: boolean;
}) {
  // `null` = still loading; `[]` after resolving is a genuine (if unlikely - there's always a
  // built-in "Default" pack) empty/error state, both shown as the same quiet empty message.
  const [packs, setPacks] = useState<CursorPackInfo[] | null>(null);
  const [importing, setImporting] = useState(false);
  const [importErr, setImportErr] = useState("");

  const loadPacks = useCallback(() => {
    setPacks(null);
    listCursorPacks().then(setPacks).catch(() => setPacks([]));
  }, []);
  useEffect(loadPacks, [loadPacks]);

  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  const handleReset = () => onChange(DEFAULT_CURSOR_SETTINGS);

  const handleImport = async () => {
    setImportErr("");
    const dir = await open({ directory: true, multiple: false, title: "Choose a cursor pack folder" }).catch(() => null);
    if (!dir || Array.isArray(dir)) return;
    setImporting(true);
    try {
      const info = await importCursorPack(dir);
      setPacks((prev) => [...(prev ?? []), info]);
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
        <Picker value={settings.style} options={STYLE_OPTS} onChange={(style) => set("style", style)} ariaLabel="Cursor style" />
      </div>
      {/* Honesty hint: this recording has no OS cursor in its pixels, so System is re-created. */}
      {settings.style === "system" && !osCursorInVideo && (
        <p className="e-lede" style={{ margin: "-2px 0 0" }}>Re-created from the recorded path — this clip was recorded without the system cursor.</p>
      )}

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
            {packs === null ? (
              <div className="e-pack-grid">
                {Array.from({ length: PACK_SKELETON_COUNT }, (_, i) => <Shimmer key={i} className="e-pack-card" />)}
              </div>
            ) : packs.length === 0 ? (
              <p className="e-lede" style={{ margin: "4px 0 0" }}>No cursor packs yet — Import a pack folder</p>
            ) : (
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
            )}
            <button type="button" className="e-upload-dashed" onClick={handleImport} disabled={importing}>
              <IconFolderPlus size={14} /> {importing ? "Importing..." : "Import pack..."}
            </button>
            {importErr && <p className="e-errline">{importErr}</p>}
          </div>

          {/* Sliders */}
          <div className="e-field">
            <span className="e-fl">Cursor Size <b>{settings.size.toFixed(2)}x</b></span>
            <Slider min={0.4} max={3.0} step={0.1} value={settings.size} onChange={(v) => set("size", v)} ariaLabel="Cursor Size" />
          </div>

          <div className="e-field">
            <span className="e-fl">Cursor Smoothness <b>{settings.smoothness.toFixed(2)}</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.smoothness} onChange={(v) => set("smoothness", v)} ariaLabel="Cursor Smoothness" />
          </div>

          <div className="e-field">
            <span className="e-fl">Path Idealization <b>{settings.path_idealize.toFixed(2)}</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.path_idealize} onChange={(v) => set("path_idealize", v)} ariaLabel="Path Idealization" />
          </div>

          <div className="e-field">
            <span className="e-fl">Motion Trail Blur <b>{settings.motion_blur.toFixed(2)}x</b></span>
            <Slider min={0.0} max={1.0} step={0.05} value={settings.motion_blur} onChange={(v) => set("motion_blur", v)} ariaLabel="Motion Trail Blur" />
          </div>

          <div className="e-field">
            <span className="e-fl">Click Bounce Intensity <b>{settings.bounce_intensity.toFixed(2)}x</b></span>
            <Slider min={0.1} max={1.0} step={0.05} value={settings.bounce_intensity} onChange={(v) => set("bounce_intensity", v)} ariaLabel="Click Bounce Intensity" />
          </div>
        </>
      )}
    </div>
  );
}
