import { useState } from "react";

export function useEditorModals(blocked: boolean) {
  const [showExport, setShowExport] = useState(false);
  const [showShortcuts, setShowShortcuts] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  return {
    dialog: {
      showExport,
      onCloseExport: () => setShowExport(false),
      showShortcuts,
      onCloseShortcuts: () => setShowShortcuts(false),
      showSettings,
      onCloseSettings: () => setShowSettings(false),
      onOpenShortcuts: () => {
        setShowSettings(false);
        setShowShortcuts(true);
      },
    },
    shortcutsOpen: showShortcuts,
    modalOpen: showExport || showShortcuts || showSettings || blocked,
    openExport: () => setShowExport(true),
    openSettings: () => setShowSettings(true),
    toggleShortcuts: () => setShowShortcuts((s) => !s),
  };
}
