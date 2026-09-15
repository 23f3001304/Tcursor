import { useEffect, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { setCapturable, getLaunchProject, getSettings } from "./shared/ipc";
import { applyTheme } from "./hud/preferences/applyTheme";
import { Hud } from "./hud/Hud";
import { Editor } from "./editor/Editor";
import { InterfaceEffects } from "./editor/effects/InterfaceEffects";

type View = { v: "hud" } | { v: "editor"; folder: string };

const safe = async (fn: () => Promise<unknown>) => {
  try {
    await fn();
  } catch {}
};

export function App() {
  const [view, setView] = useState<View>({ v: "hud" });
  const win = getCurrentWindow();

  const openEditor = async (folder: string) => {
    const sw = window.screen.availWidth,
      sh = window.screen.availHeight;
    const w = Math.min(1440, sw - 120),
      h = Math.min(900, sh - 120);
    await safe(() => win.setResizable(true));
    await safe(() => win.setAlwaysOnTop(false));
    await safe(() => win.setMinSize(new LogicalSize(880, 560)));
    await safe(() => win.setSize(new LogicalSize(w, h)));
    await safe(() => win.center());
    await safe(() => setCapturable(true));
    setView({ v: "editor", folder });
  };

  const closeEditor = async () => {
    await safe(() => setCapturable(false));
    await safe(() => win.setAlwaysOnTop(true));
    await safe(() => win.setResizable(false));
    await safe(() => win.setMinSize(null));
    await safe(() => win.setSize(new LogicalSize(980, 132)));
    await safe(() => win.center());
    setView({ v: "hud" });
  };

  useEffect(() => {
    getSettings()
      .then((s) => applyTheme(s.ui.theme, s.ui.accent))
      .catch(() => {});
    getLaunchProject()
      .then((folder) => {
        if (folder) void openEditor(folder);
      })
      .catch(() => {});
  }, []);

  return view.v === "editor" ? (
    <>
      <Editor folder={view.folder} onClose={closeEditor} />
      <InterfaceEffects />
    </>
  ) : (
    <Hud onEdit={openEditor} />
  );
}
