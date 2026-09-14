import { useEffect, useRef, useState } from "react";
import { MotionConfig } from "motion/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { useDevices } from "./hooks/useDevices";
import { useHudWindowSize, takeWidth } from "./hooks/useHudWindowSize";
import { useRecordingTimer } from "./hooks/useRecordingTimer";
import { useWebcamPreview } from "./hooks/useWebcamPreview";
import { useCameraDevices } from "./hooks/useCameraDevices";
import { useAudioLevels } from "./hooks/useAudioLevels";
import { useRecordingFlow } from "./hooks/useRecordingFlow";
import { useSourceSwitch } from "./hooks/useSourceSwitch";
import { IdleCard, type Toggles } from "./components/IdleCard";
import { TakeBar } from "./components/TakeBar";
import { SourcesSheet } from "./components/SourcesSheet";
import { StateSwap } from "./components/StateSwap";
import { cleanDeviceLabel } from "./devices/selectDevices";
import { getSettings, openProject } from "../lib/ipc";
import { applyTheme } from "./preferences/applyTheme";
import type { InterfaceSettings, ThemeMode } from "./settings/settings";
import { useWebcamRecorder } from "./hooks/useWebcamRecorder";
import { Settings } from "./settings/SettingsPanel";
import { Preferences } from "./preferences/Preferences";

export function Hud({ onEdit }: { onEdit?: (folder: string) => Promise<void> }) {
  const { displays, mics, sel, setSel } = useDevices();
  const [camId, setCamId] = useState<string | null>(null); const [camOn, setCamOn] = useState(true);
  // `useRecordingFlow` needs a stable `camStream` getter, but `cam`'s own `enabled` (below) needs
  // `saving`, which only exists once `useRecordingFlow` returns - broken via this ref instead of a
  // circular reference; `webcam.start` only ever calls it later, once `cam` has set the real getter.
  const camStreamRef = useRef<() => MediaStream | null>(() => null);
  // Shared by the idle card and the take pill's Sources sheet - the two are never on screen at the
  // same time, so one open-menu id and one display-list flip serve both.
  const [menu, setMenu] = useState<string | null>(null);
  const [sheet, setSheet] = useState(false);
  const [sources, setSources] = useState(false);
  const [micOn, setMicOn] = useState(true); const [sysOn, setSysOn] = useState(false); const [gameMode, setGameMode] = useState(false);
  const [exporting, setExporting] = useState(false); const [pct, setPct] = useState(0);
  const [exportErr, setExportErr] = useState<string | null>(null);
  // Settings and Preferences are a sheet inside the idle card, exactly like the display picker:
  // this is which one (if any) has the card's body, never a second window state.
  const [panel, setPanel] = useState<"settings" | "preferences" | null>(null);
  // True once the take pill is the SHOWN state (the idle bar has frosted out), false once the bar
  // is: the window is sized for whichever is on screen, not for what the flow just switched to.
  const [takeShown, setTakeShown] = useState(false);
  const lastFolder = useRef<string>("");
  const themeRef = useRef<{ theme: ThemeMode; accent: [number, number, number] }>({ theme: "light", accent: [239, 68, 68] });
  const webcam = useWebcamRecorder();
  // Owns record/stop/preprocess state + the toggle/togglePause handlers (see useRecordingFlow's
  // doc comment). `stopForClose` is the graceful stop `handleClose` below runs before actually
  // closing (R6) - `lib.rs`'s Rust `CloseRequested` guard is the backstop if that never runs.
  const { recording, paused, saving, savePct, err, toggle, togglePause, stopForClose } = useRecordingFlow({
    micOn, micId: sel.micId, displayId: sel.displayId, sysOn, gameMode, camOn,
    camStream: () => camStreamRef.current(), webcam, onEdit, lastFolderRef: lastFolder,
  });
  // Saving (item 4): the stream is released here (and the tile hidden below) the instant Stop's
  // finalize begins - re-acquired automatically once `saving` goes false (this `enabled` gate).
  const cam = useWebcamPreview(camId, camOn && !saving);
  camStreamRef.current = cam.stream;
  const cameras = useCameraDevices(cam.on ? 1 : 0);
  const elapsed = useRecordingTimer(recording, paused);
  // Mic AND system levels, from the Rust capture itself - no second mic stream in the webview.
  const audio = useAudioLevels(recording && !paused && (micOn || sysOn));
  const win = getCurrentWindow();

  // One window size for every idle state, panels included: the hook tracks the dropdown height and
  // the take pill, and fixes the cold-launch centering bug (L1) - see its own doc comment.
  // `saving` wins over `recording` (mode, item 3 - see useHudWindowSize.md).
  const take = recording || saving;
  // Mid-take source switching (2026-09-14): the Sources sheet's pickers run through this, which
  // applies each change to the RUNNING take and to the selection the idle card shows. Its own
  // failures ride the take pill's existing warning slot, next to the flow's.
  const src = useSourceSwitch({
    recording, camOn, micOn, camId, camStream: () => camStreamRef.current(), webcam,
    setCamId, setMicId: (id) => setSel((s) => ({ ...s, micId: id })),
    setDisplayId: (id) => setSel((s) => ({ ...s, displayId: id })),
  });
  const warn = err ?? src.err;
  // Hidden while Saving: there is no take left to switch anything on.
  const sourcesOpen = sources && !saving;
  // The pill shows only the sources that are on, so its window is sized per take (`takeWidth`);
  // `sources` grows the window's HEIGHT for the sheet under it, never its width.
  const pill = { cam: camOn, meter: micOn || sysOn, warn: warn != null, sources: sourcesOpen };
  useHudWindowSize(menu, takeShown ? (saving ? "saving" : "recording") : "idle", pill);

  // A take ending takes the sheet (and anything it had open) with it, so the idle card never
  // comes back mid-flip, with a stale menu hanging off it, or on a panel nobody asked for.
  useEffect(() => { if (!recording) { setSources(false); setMenu(null); setSheet(false); setPanel(null); } }, [recording]);

  // Escape leaves a panel the way its own Back arrow does. `defaultPrevented` keeps the hotkey
  // capture's Escape (SettingsHotkeys) on cancelling the capture instead of closing the panel.
  useEffect(() => {
    if (!panel) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape" && !e.defaultPrevented) setPanel(null); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [panel]);

  // Apply theme on mount and keep System mode tracking the OS preference.
  useEffect(() => {
    getSettings().then(s => {
      themeRef.current = { theme: s.ui.theme, accent: s.ui.accent };
      applyTheme(s.ui.theme, s.ui.accent);
    });
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const onMqChange = () => applyTheme(themeRef.current.theme, themeRef.current.accent);
    mq.addEventListener("change", onMqChange);
    return () => mq.removeEventListener("change", onMqChange);
  }, []);

  // Opening a panel takes the card's body, so anything else hanging off the card goes with it -
  // an open menu would otherwise leave the window tall for a list nobody can see any more.
  function openPanel(p: "settings" | "preferences") { setMenu(null); setSheet(false); setPanel(p); }
  const closePanel = () => setPanel(null);

  // R6: gracefully stop + save mid-recording, never destroy the take, no confirm dialog.
  // `stopForClose` no-ops instantly otherwise, so this is still one click in the common case.
  async function handleClose() { await stopForClose(); win.close(); }

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("export-progress", e => { setPct(e.payload); setExportErr(null); }));
    unsubs.push(listen<string>("export-done", e => {
      setExporting(false);
      // `e.payload` is now the exported file's own absolute path (`<folder>/final.<ext>`, see
      // run.rs) - no more hardcoded `\final.mp4` guess, which was wrong for a webm/gif export.
      revealItemInDir(e.payload).catch(() => {});
    }));
    unsubs.push(listen<string>("export-error", e => {
      setExporting(false);
      // The message is what surfaces the failure now (M3) - the reveal below is best-effort on
      // top of it, and stays a no-op for a project opened via Open Project (`lastFolder` = "").
      setExportErr(e.payload);
      if (lastFolder.current) revealItemInDir(`${lastFolder.current}\\video.mp4`).catch(() => {});
    }));
    return () => { unsubs.forEach(u => u.then(f => f())); };
  }, []);

  // Opens a *.tcursor file picker and routes to the editor via the same onEdit path Stop uses -
  // awaited (onEdit is promisified, fix round 1 item 2) so a rejection lands in the same catch as a cancelled dialog.
  async function openExistingProject() {
    try { await onEdit?.(await openProject()); } catch { /* dialog cancelled - nothing to surface */ }
  }
  const tg = (id: string) => setMenu((m) => (m === id ? null : id));
  const setters = { camOn: setCamOn, micOn: setMicOn, sysOn: setSysOn, gameMode: setGameMode };
  const flip = (k: keyof Toggles) => setters[k]((v) => !v);
  const camOpts = cameras.length ? cameras.map((c) => ({ id: c.id, label: cleanDeviceLabel(c.label) })) : [{ id: "", label: "Camera" }];
  const micOpts = mics.map((m) => ({ id: m.id, label: cleanDeviceLabel(m.label) }));
  const banner = warn ?? exportErr;
  // Every Sources pick applies at once and closes the whole sheet - no Apply, nothing to confirm.
  const picked = (run: () => void) => { run(); setMenu(null); setSheet(false); setSources(false); };
  // The open panel, handed to the card, which frosts it into its body in place of the sources.
  const onUi = (ui: InterfaceSettings) => { themeRef.current = { theme: ui.theme, accent: ui.accent }; applyTheme(ui.theme, ui.accent); };
  const panelBody = panel === "preferences" ? <Preferences onClose={closePanel} onUiChange={onUi} />
    : panel === "settings" ? <Settings onClose={closePanel} /> : null;

  return (
    <MotionConfig reducedMotion="user">
    <div className={`hud${takeShown ? " as-take" : ""}${sourcesOpen ? " has-sources" : ""}`}
      style={takeShown ? { width: takeWidth(pill) } : undefined}>
      <StateSwap take={take} onSettled={setTakeShown}
        pill={<>
          <TakeBar paused={paused} saving={saving} savePct={savePct} elapsed={elapsed} err={warn} micOn={micOn} sysOn={sysOn} live={audio.live}
            read={audio.read} camRef={cam.ref} camOn={camOn} camLive={cam.on} toggle={toggle} togglePause={togglePause}
            sources={sourcesOpen} onSources={() => { setSources((s) => !s); setMenu(null); setSheet(false); src.clearErr(); }} />
          {sourcesOpen && <SourcesSheet targets={displays} displayId={sel.displayId ?? ""}
            onTarget={(id) => picked(() => void src.switchDisplay(id))}
            cameras={camOpts} camId={camId ?? cameras[0]?.id ?? ""} onCam={(id) => picked(() => void src.switchCamera(id || null))}
            mics={micOpts} micId={sel.micId ?? ""} onMic={(id) => picked(() => void src.switchMic(id))}
            menu={menu} onMenu={tg} sheet={sheet} onSheet={setSheet} />}
        </>}
        idle={<IdleCard banner={banner} exporting={exporting} pct={pct} onOpenProject={openExistingProject}
          onPreferences={() => openPanel("preferences")} onSettings={() => openPanel("settings")}
          onMinimize={() => void win.minimize()} onClose={() => void handleClose()}
          camRef={cam.ref} camLive={cam.on} cameras={camOpts} camId={camId ?? cameras[0]?.id ?? ""}
          onCam={(id) => { setCamId(id || null); setMenu(null); }}
          targets={displays} displayId={sel.displayId ?? ""} onTarget={(id) => setSel({ ...sel, displayId: id })}
          mics={micOpts} micId={sel.micId ?? ""}
          onMic={(id) => { setSel({ ...sel, micId: id }); setMenu(null); }}
          menu={menu} onMenu={tg} sheet={sheet} onSheet={setSheet} panel={panel} panelBody={panelBody}
          toggles={{ camOn, micOn, sysOn, gameMode }} onToggle={flip} onRecord={toggle} />} />
    </div>
    </MotionConfig>
  );
}
