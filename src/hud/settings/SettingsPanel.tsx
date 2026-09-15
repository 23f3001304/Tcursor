import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { getSettings, setSettings } from "../../shared/ipc";
import type { Settings as S } from "./settings";
import { Back } from "../components/icons";
import { SettingsZoom } from "./SettingsZoom";
import { SettingsHotkeys } from "./SettingsHotkeys";
import { SettingsClickFx } from "./SettingsClickFx";
import { SettingsCursor } from "./SettingsCursor";
import { Range } from "./SettingsRange";
import "./settings.css";

type Tab = "zoom" | "cursor" | "keys" | "fx";
const TABS: { id: Tab; label: string }[] = [
  { id: "zoom", label: "Zoom" },
  { id: "cursor", label: "Cursor" },
  { id: "keys", label: "Keys" },
  { id: "fx", label: "FX" },
];

export function Settings({ onClose }: { onClose: () => void }) {
  const [draft, setDraft] = useState<S | null>(null);
  const [tab, setTab] = useState<Tab>("zoom");
  useEffect(() => {
    getSettings()
      .then(setDraft)
      .catch(() => {});
  }, []);
  function patch(next: S) {
    setDraft(next);
    setSettings(next).catch(() => {});
  }
  if (!draft) return null;

  return (
    <div className="settings">
      <div className="settings-head" data-tauri-drag-region>
        <button className="winbtn" title="Back" onClick={onClose}>
          <Back />
        </button>
        <span className="settings-title">Settings</span>
      </div>
      <div className="tabs">
        {TABS.map((t) => (
          <button key={t.id} className={`tab ${tab === t.id ? "on" : ""}`} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </div>
      <div className="settings-body">
        <AnimatePresence mode="wait">
          <motion.div
            className="tab-panel"
            key={tab}
            initial={{ opacity: 0, x: 8 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -8 }}
            transition={{ duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            {tab === "zoom" && (
              <>
                <SettingsZoom value={draft.zoom} onChange={(zoom) => patch({ ...draft, zoom })} />
                <Range
                  label="Mic sync offset"
                  hint="- earlier - + later"
                  value={draft.audio_offset_ms}
                  min={-300}
                  max={300}
                  step={10}
                  onChange={(v) => patch({ ...draft, audio_offset_ms: Math.round(v) })}
                  fmt={(v) => `${v > 0 ? "+" : ""}${Math.round(v)} ms`}
                />
              </>
            )}
            {tab === "cursor" && (
              <SettingsCursor value={draft.cursor} onChange={(cursor) => patch({ ...draft, cursor })} />
            )}
            {tab === "keys" && (
              <SettingsHotkeys value={draft.hotkeys} onChange={(hotkeys) => patch({ ...draft, hotkeys })} />
            )}
            {tab === "fx" && (
              <SettingsClickFx value={draft.clickfx} onChange={(clickfx) => patch({ ...draft, clickfx })} />
            )}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
