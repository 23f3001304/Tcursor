import { useEffect, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { setCapturable, getLaunchProject } from "./lib/ipc";
import { Hud } from "./hud/Hud";
import { Editor } from "./editor/Editor";

type View = { v: "hud" } | { v: "editor"; folder: string };

/** Run a window op without letting its failure abort the others. */
const safe = async (fn: () => Promise<unknown>) => { try { await fn(); } catch { /* ignore */ } };

/** Top-level view switch: the floating recorder HUD, or the windowed editor for a
 *  finished recording, both in this single (transparent) window. Switching is React
 *  state - never window-label routing - so the editor always renders. Opening the editor
 *  makes the window resizable, sizes it to a comfortable centered window (never
 *  fullscreen), and opts back into capture; setSize/center are the ops that work on this
 *  transparent window where maximize/fullscreen no-op. */
export function App() {
  const [view, setView] = useState<View>({ v: "hud" });
  const win = getCurrentWindow();

  const openEditor = async (folder: string) => {
    const sw = window.screen.availWidth, sh = window.screen.availHeight;
    const w = Math.min(1440, sw - 120), h = Math.min(900, sh - 120);
    await safe(() => win.setResizable(true)); // HUD window is non-resizable
    await safe(() => win.setAlwaysOnTop(false));
    await safe(() => win.setMinSize(new LogicalSize(880, 560))); // floor below which the editor breaks
    await safe(() => win.setSize(new LogicalSize(w, h)));
    await safe(() => win.center());
    await safe(() => setCapturable(true)); // editor should appear in screenshots/recordings
    setView({ v: "editor", folder });
  };

  const closeEditor = async () => {
    await safe(() => setCapturable(false)); // re-hide the window from capture
    await safe(() => win.setAlwaysOnTop(true));
    await safe(() => win.setResizable(false));
    await safe(() => win.setMinSize(null)); // clear the editor floor so the HUD bar can shrink back
    await safe(() => win.setSize(new LogicalSize(980, 132))); // restore the HUD bar
    await safe(() => win.center());
    setView({ v: "hud" });
  };

  // Cold-start file association: if this process was launched by double-clicking a `.tcursor`
  // file, the backend resolved its folder in `setup()`; route straight to the editor instead of
  // flashing the HUD first. A normal launch resolves `null` here and nothing happens. Warm-launch
  // (the app already running when another `.tcursor` is opened) is not covered - see the Rust
  // `LaunchProject` doc comment.
  useEffect(() => {
    getLaunchProject().then((folder) => { if (folder) void openEditor(folder); }).catch(() => {});
  }, []);

  return view.v === "editor"
    ? <Editor folder={view.folder} onClose={closeEditor} />
    : <Hud onEdit={openEditor} />;
}
