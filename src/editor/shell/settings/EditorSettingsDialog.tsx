import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
import { IconKeyboard, IconX } from "@tabler/icons-react";
import type { EditDoc } from "../../../shared/edit";
import { ZoomDefaultsSection } from "./ZoomDefaultsSection";
import { MotionSection } from "./MotionSection";
import { ScreenSection } from "./ScreenSection";
import { InterfaceSection } from "./InterfaceSection";
import { mirrorUiToApp } from "./applyUi";

export function EditorSettingsDialog({
  open,
  settings,
  onClose,
  onSaveSettings,
  onOpenShortcuts,
  onApplyToAll,
}: {
  open: boolean;
  settings: EditDoc["settings"];
  onClose: () => void;
  onSaveSettings: (next: EditDoc["settings"]) => void;
  onOpenShortcuts: () => void;
  onApplyToAll: () => void;
}) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  const setZoom = (zoom: EditDoc["settings"]["zoom"]) => onSaveSettings({ ...settings, zoom });
  const setScreen = (screen: EditDoc["settings"]["appearance"]["screen"]) =>
    onSaveSettings({ ...settings, appearance: { ...settings.appearance, screen } });
  const setUi = (ui: EditDoc["settings"]["ui"]) => {
    onSaveSettings({ ...settings, ui });
    void mirrorUiToApp(ui);
  };
  const setMotion = (motion: EditDoc["settings"]["motion"]) => onSaveSettings({ ...settings, motion });
  const setSmoothing = (ms: number) => setZoom({ ...settings.zoom, camera_smoothing_ms: ms });

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="e-modal-scrim"
          onPointerDown={onClose}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14 }}
        >
          <motion.div
            className="e-modal e-settings-modal"
            onPointerDown={(e) => e.stopPropagation()}
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            <div className="e-export-head">
              <h3 className="e-modal-title">Project settings</h3>
              <button type="button" className="e-gst" title="Close" onClick={onClose}>
                <IconX size={16} />
              </button>
            </div>

            <ZoomDefaultsSection value={settings.zoom} onChange={setZoom} />
            <MotionSection
              value={settings.motion}
              onChange={setMotion}
              smoothingMs={settings.zoom.camera_smoothing_ms}
              onSmoothingChange={setSmoothing}
              onApplyToAll={onApplyToAll}
            />
            <ScreenSection value={settings.appearance.screen} onChange={setScreen} />
            <InterfaceSection value={settings.ui} onChange={setUi} />

            <div className="e-sec">
              <div className="e-secrow">
                <span>Keyboard shortcuts</span>
                <button type="button" className="e-modal-btn" onClick={onOpenShortcuts}>
                  <IconKeyboard size={14} />
                  View
                </button>
              </div>
            </div>

            <div className="e-sec">
              <p className="e-lede" style={{ margin: 0 }}>
                Capture settings (hotkeys, devices, game mode) live in the recorder. They apply at record
                time.
              </p>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
