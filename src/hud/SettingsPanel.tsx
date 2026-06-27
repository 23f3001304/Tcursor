import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { getSettings, setSettings } from "../lib/ipc";
import type { Settings as S } from "./settings";
import { Back } from "./icons";
import { SettingsZoom } from "./SettingsZoom";
import { SettingsHotkeys } from "./SettingsHotkeys";
import { SettingsClickFx } from "./SettingsClickFx";
import { Field, Slider } from "./SettingsControls";
import "./settings.css";

type Tab = "zoom" | "keys" | "fx";
const TABS: { id: Tab; label: string }[] = [
  { id: "zoom", label: "Zoom" }, { id: "keys", label: "Keys" }, { id: "fx", label: "FX" },
];

/** Small settings box: a back arrow, segmented tabs, and one section at a time.
 *  Loads the persisted settings into a draft and writes every change back. */
export function Settings({ onClose }: { onClose: () => void }) {
  const [draft, setDraft] = useState<S | null>(null);
  const [tab, setTab] = useState<Tab>("zoom");
  useEffect(() => { getSettings().then(setDraft).catch(() => {}); }, []);
  function patch(next: S) { setDraft(next); setSettings(next).catch(() => {}); }
  if (!draft) return null;

  return (
    <div className="settings">
      <div className="settings-head" data-tauri-drag-region>
        <button className="winbtn" title="Back" onClick={onClose}><Back /></button>
        <span className="settings-title">Settings</span>
      </div>
      <div className="tabs">
        {TABS.map((t) => (
          <button key={t.id} className={`tab ${tab === t.id ? "on" : ""}`} onClick={() => setTab(t.id)}>{t.label}</button>
        ))}
      </div>
      <div className="settings-body">
        <AnimatePresence mode="wait">
        <motion.div className="tab-panel" key={tab}
          initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }}
          transition={{ duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
          {tab === "zoom" && (
            <>
              <SettingsZoom value={draft.zoom} onChange={(zoom) => patch({ ...draft, zoom })} />
              <Field label="Mic sync offset" hint="- earlier · + later">
                <Slider value={draft.audio_offset_ms} min={-300} max={300} step={10}
                  onChange={(v) => patch({ ...draft, audio_offset_ms: Math.round(v) })}
                  fmt={(v) => `${v > 0 ? "+" : ""}${Math.round(v)} ms`} />
              </Field>
            </>
          )}
          {tab === "keys" && <SettingsHotkeys value={draft.hotkeys} onChange={(hotkeys) => patch({ ...draft, hotkeys })} />}
          {tab === "fx" && <SettingsClickFx value={draft.clickfx} onChange={(clickfx) => patch({ ...draft, clickfx })} />}
        </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
