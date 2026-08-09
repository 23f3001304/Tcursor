import { useEffect, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { useDevices } from "./hooks/useDevices";
import { useRecordingTimer } from "./hooks/useRecordingTimer";
import { useWebcamPreview } from "./hooks/useWebcamPreview";
import { useCameraDevices } from "./hooks/useCameraDevices";
import { useMicWaveform } from "./hooks/useMicWaveform";
import { useRecordingFlow } from "./hooks/useRecordingFlow";
import { formatTimer } from "./components/formatTimer";
import { Dropdown } from "./components/Dropdown";
import { TargetPicker } from "./devices/TargetPicker";
import { Grip, Mic, MicOff, Speaker, SpeakerOff, Camera, CameraOff, MinIcon, CloseIcon, Gear, Gamepad, Palette, FolderOpen } from "./components/icons";
import { getSettings, openProject } from "../lib/ipc";
import { TcursorMark } from "../lib/TcursorMark";
import { applyTheme } from "./preferences/applyTheme";
import type { ThemeMode } from "./settings/settings";
import { useWebcamRecorder } from "./hooks/useWebcamRecorder";
import { Settings } from "./settings/SettingsPanel";
import { Preferences } from "./preferences/Preferences";
import { morphWindow } from "./components/morph";

const WIDTH = 980;

export function Hud({ onEdit }: { onEdit?: (folder: string) => void }) {
  const { displays, mics, sel, setSel } = useDevices();
  const [camId, setCamId] = useState<string | null>(null); const [camOn, setCamOn] = useState(true);
  const cam = useWebcamPreview(camId, camOn);
  const cameras = useCameraDevices(cam.on ? 1 : 0);
  const [menu, setMenu] = useState<string | null>(null);
  const [micOn, setMicOn] = useState(true); const [sysOn, setSysOn] = useState(false); const [gameMode, setGameMode] = useState(false);
  const [exporting, setExporting] = useState(false); const [pct, setPct] = useState(0);
  const [panel, setPanel] = useState<"settings" | "preferences" | null>(null);
  const [barShown, setBarShown] = useState(true);
  const lastFolder = useRef<string>("");
  const themeRef = useRef<{ theme: ThemeMode; accent: [number, number, number] }>({ theme: "light", accent: [239, 68, 68] });
  // Task 39's feel knob - gates whether the titlebar mark flows/pulses at all. Real React state
  // (unlike theme/accent, applied imperatively via CSS vars) because it's a prop TcursorMark reads.
  const [animatedBrand, setAnimatedBrand] = useState(true);
  const webcam = useWebcamRecorder();
  // Owns record/stop/preprocess state + the toggle/togglePause handlers - see useRecordingFlow's
  // doc comment for why preprocessing is awaited (with progress) between Stop and onEdit.
  const { recording, paused, saving, savePct, err, toggle, togglePause } = useRecordingFlow({
    micOn, micId: sel.micId, displayId: sel.displayId, sysOn, gameMode, camOn,
    camStream: cam.stream, webcam, onEdit, lastFolderRef: lastFolder,
  });
  const elapsed = useRecordingTimer(recording, paused);
  const levels = useMicWaveform(recording && !paused);
  const win = getCurrentWindow();

  const BOX_W = 360, BOX_H = 440; // 16px body top-padding + 420 box + a little slack
  const barSize = () => new LogicalSize(WIDTH, menu ? 430 : 132);
  // The box<->bar window resize is driven by openSettings and the settings exit
  // (restoreBar via onExitComplete) so the spring-out is never clipped; this
  // effect only tracks the dropdown height while the bar is showing.
  useEffect(() => { if (barShown) win.setSize(barSize()); }, [menu]);

  // Apply theme on mount and keep System mode tracking the OS preference.
  useEffect(() => {
    getSettings().then(s => {
      themeRef.current = { theme: s.ui.theme, accent: s.ui.accent };
      applyTheme(s.ui.theme, s.ui.accent);
      setAnimatedBrand(s.ui.animated_brand);
    });
    const mq = matchMedia("(prefers-color-scheme: dark)");
    const onMqChange = () => applyTheme(themeRef.current.theme, themeRef.current.accent);
    mq.addEventListener("change", onMqChange);
    return () => mq.removeEventListener("change", onMqChange);
  }, []);

  function openPanel(p: "settings" | "preferences") { setBarShown(false); setPanel(p); void morphWindow(WIDTH, menu ? 430 : 132, BOX_W, BOX_H, 200); }
  function restoreBar() { void morphWindow(BOX_W, BOX_H, WIDTH, menu ? 430 : 132, 200).then(() => setBarShown(true)); }

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("export-progress", e => setPct(e.payload)));
    unsubs.push(listen<string>("export-done", e => {
      setExporting(false);
      // `e.payload` is now the exported file's own absolute path (`<folder>/final.<ext>`, see
      // run.rs) - no more hardcoded `\final.mp4` guess, which was wrong for a webm/gif export.
      revealItemInDir(e.payload).catch(() => {});
    }));
    unsubs.push(listen<string>("export-error", () => {
      setExporting(false);
      revealItemInDir(`${lastFolder.current}\\video.mp4`).catch(() => {});
    }));
    return () => { unsubs.forEach(u => u.then(f => f())); };
  }, []);

  // Opens a *.tcursor file picker and routes to the editor via the same onEdit path Stop uses.
  async function openExistingProject() {
    try { onEdit?.(await openProject()); } catch { /* dialog cancelled - nothing to surface */ }
  }
  const tg = (id: string) => setMenu((m) => (m === id ? null : id));
  const camOpts = cameras.length ? cameras.map((c) => ({ id: c.id, label: c.label })) : [{ id: "", label: "Camera" }];

  return (
    <MotionConfig reducedMotion="user">
    <div className={`hud ${barShown ? "" : "as-box"}`}>
      <AnimatePresence onExitComplete={restoreBar}>
        {panel && (
          <motion.div key={panel} className="settings-wrap" style={{ originX: 1, originY: 0 }}
            initial={{ opacity: 0, scale: 0.97 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 0.98 }}
            transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}>
            {panel === "settings"
              ? <Settings onClose={() => setPanel(null)} />
              : <Preferences onClose={() => setPanel(null)} onUiChange={(ui) => {
                  themeRef.current = { theme: ui.theme, accent: ui.accent };
                  applyTheme(ui.theme, ui.accent);
                  setAnimatedBrand(ui.animated_brand);
                }} />}
          </motion.div>
        )}
      </AnimatePresence>
      {barShown && (
        <>
          <div className="titlebar" data-tauri-drag-region>
            <span className="brand"><span className="brand-mark"><TcursorMark size={11} dotColor="var(--accent, #ef4444)" state={animatedBrand && recording ? "recording" : "idle"} /></span>TCursor</span>
            {err && <span style={{ color: "#ff6b6b", fontSize: 11, marginLeft: 10 }} title={err}>⚠ recording failed: {err}</span>}
            <span className="winctrls">
              {!recording && !exporting && (<>
                <button className="winbtn" title="Open Project" onClick={openExistingProject} disabled={saving}><FolderOpen /></button>
                <button className="winbtn" title="Preferences" onClick={() => openPanel("preferences")}><Palette /></button>
                <button className="winbtn gear" title="Settings" onClick={() => openPanel("settings")}><Gear /></button>
              </>)}
              <button className="winbtn" title="Minimize" onClick={() => win.minimize()}><MinIcon /></button>
              <button className="winbtn close" title="Close" onClick={() => win.close()}><CloseIcon /></button>
            </span>
          </div>

          <div className="row">
            <div className="grip" data-tauri-drag-region><Grip /></div>

            <div className="camtoggle" title="Camera preview">
              <video ref={cam.ref} className={`cam ${camOn && cam.on ? "" : "off"}`} autoPlay muted playsInline />
              {!(camOn && cam.on) && <span className="camoff"><Camera /></span>}
            </div>

            {exporting ? (
              <span className="exporting">Exporting… {pct}%</span>
            ) : saving ? (
              <span className="exporting saving">Saving… {savePct}%</span>
            ) : !recording ? (
              <>
                <Dropdown icon={<Camera />} value={camId ?? cameras[0]?.id ?? ""} options={camOpts}
                  open={menu === "cam"} onToggle={() => tg("cam")} onPick={(id) => { setCamId(id || null); setMenu(null); }} />
                <div className="divider" />
                <TargetPicker targets={displays} value={sel.displayId ?? ""}
                  open={menu === "screen"} onToggle={() => tg("screen")} onPick={(id) => { setSel({ ...sel, displayId: id }); setMenu(null); }} />
                <Dropdown icon={<Mic />} value={sel.micId ?? ""} options={mics.map((m) => ({ id: m.id, label: m.label }))}
                  open={menu === "mic"} onToggle={() => tg("mic")} onPick={(id) => { setSel({ ...sel, micId: id }); setMenu(null); }} />
                <button className={`toggle ${camOn ? "on" : ""}`} title={camOn ? "Camera on" : "Camera off"} onClick={() => setCamOn(v => !v)}>{camOn ? <Camera /> : <CameraOff />}</button>
                <button className={`toggle ${micOn ? "on" : ""}`} title={micOn ? "Microphone on" : "Microphone off"} onClick={() => setMicOn(v => !v)}>{micOn ? <Mic /> : <MicOff />}</button>
                <button className={`toggle ${sysOn ? "on" : ""}`} title={sysOn ? "System audio on" : "System audio off"} onClick={() => setSysOn(v => !v)}>{sysOn ? <Speaker /> : <SpeakerOff />}</button>
                <button className={`toggle ${gameMode ? "on" : ""}`} title={gameMode ? "Compatibility encoder (on): legacy CPU recording" : "Compatibility encoder: switch on only if GPU recording has issues"} onClick={() => setGameMode(v => !v)}><Gamepad /></button>
              </>
            ) : (
              <div className="recmeter">
                <span className="ico"><Mic /></span>
                <div className="wave">{levels.map((l, i) => <span key={i} style={{ height: `${3 + l * 18}px` }} />)}</div>
              </div>
            )}

            <div className="spacer" />

            {recording && <span className="timer">{formatTimer(elapsed)}</span>}
            {recording && <button className="btn" onClick={togglePause}>{paused ? "Resume" : "Pause"}</button>}
            <button className={`btn rec ${recording ? "is-rec" : ""}`} onClick={toggle} disabled={exporting || saving}>
              <span className="dot" />{saving ? "Saving…" : recording ? "Stop" : "Record"}
            </button>
          </div>
        </>
      )}
    </div>
    </MotionConfig>
  );
}
