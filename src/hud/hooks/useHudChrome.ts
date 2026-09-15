import { useEffect, useState } from "react";

export type HudPanel = "settings" | "preferences" | null;

export function useHudChrome(recording: boolean) {
  const [menu, setMenu] = useState<string | null>(null);
  const [sheet, setSheet] = useState(false);
  const [sources, setSources] = useState(false);
  const [panel, setPanel] = useState<HudPanel>(null);

  useEffect(() => {
    if (!recording) {
      setSources(false);
      setMenu(null);
      setSheet(false);
      setPanel(null);
    }
  }, [recording]);

  useEffect(() => {
    if (!panel) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) setPanel(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [panel]);

  return {
    menu,
    sheet,
    sources,
    panel,
    setSheet,
    setSources,
    toggleMenu: (id: string) => setMenu((m) => (m === id ? null : id)),
    closeMenu: () => setMenu(null),
    openPanel: (p: Exclude<HudPanel, null>) => {
      setMenu(null);
      setSheet(false);
      setPanel(p);
    },
    closePanel: () => setPanel(null),
    // Every source pick closes the whole chrome, so the sheet never outlives the choice made in it.
    picked: (run: () => void) => {
      run();
      setMenu(null);
      setSheet(false);
      setSources(false);
    },
    openSources: (clearErr: () => void) => {
      setSources((s) => !s);
      setMenu(null);
      setSheet(false);
      clearErr();
    },
  };
}
