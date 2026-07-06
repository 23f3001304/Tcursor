import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { getSettings, setSettings } from "../../lib/ipc";
import type { Settings as S, ThemeMode } from "../settings/settings";
import { Back } from "../components/icons";
import { SettingsInterface } from "../settings/SettingsInterface";
import { SettingsAppearance } from "../settings/SettingsAppearance";

type Tab = "interface" | "layout";
const TABS: { id: Tab; label: string }[] = [{ id: "interface", label: "Interface" }, { id: "layout", label: "Layout" }];

export function Preferences({ onClose, onThemeChange }: {
  onClose: () => void;
  onThemeChange: (theme: ThemeMode, accent: [number, number, number]) => void;
}) {
  const [draft, setDraft] = useState<S | null>(null);
  const [tab, setTab] = useState<Tab>("interface");
  useEffect(() => { getSettings().then(setDraft).catch(() => {}); }, []);

  function patch(next: S) {
    setDraft(next);
    setSettings(next).catch(() => {});
  }

  if (!draft) return null;

  return (
    <div className="settings">
      <div className="settings-head" data-tauri-drag-region>
        <button className="winbtn" title="Back" onClick={onClose}><Back /></button>
        <span className="settings-title">Preferences</span>
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
          <motion.div className="tab-panel" key={tab}
            initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }}
            transition={{ duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
            {tab === "interface" && (
              <SettingsInterface
                value={draft.ui}
                onChange={(ui) => {
                  patch({ ...draft, ui });
                  onThemeChange(ui.theme, ui.accent);
                }}
              />
            )}
            {tab === "layout" && <SettingsAppearance value={draft.appearance} onChange={(appearance) => patch({ ...draft, appearance })} />}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
