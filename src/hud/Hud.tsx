import { useEffect, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { useDevices } from "./hooks/useDevices";
import { useHudWindowSize, WIDTH } from "./hooks/useHudWindowSize";
import { useRecordingTimer } from "./hooks/useRecordingTimer";
import { useWebcamPreview } from "./hooks/useWebcamPreview";
import { useCameraDevices } from "./hooks/useCameraDevices";
import { useAudioLevels } from "./hooks/useAudioLevels";
import { useRecordingFlow } from "./hooks/useRecordingFlow";
import { Dropdown } from "./components/Dropdown";
import { RecMeter } from "./components/RecMeter";
import { CamTile } from "./components/CamTile";
import { RecordingControls } from "./components/RecordingControls";
import { TargetPicker } from "./devices/TargetPicker";
import { cleanDeviceLabel } from "./devices/selectDevices";
import { Grip, Mic, MicOff, Speaker, SpeakerOff, Camera, CameraOff, MinIcon, CloseIcon, Gear, Gamepad, Palette, FolderOpen } from "./components/icons";
import { getSettings, openProject } from "../lib/ipc";
import { TcursorMark } from "../lib/TcursorMark";
import { applyTheme } from "./preferences/applyTheme";
import type { ThemeMode } from "./settings/settings";
import { useWebcamRecorder } from "./hooks/useWebcamRecorder";
import { Settings } from "./settings/SettingsPanel";
import { Preferences } from "./preferences/Preferences";
import { morphWindow } from "./components/morph";
// design/premium-pass D6: press spring for the toggle cluster (Record + Pause/Resume already have their own - see below/hud.css).
const TOGGLE_PRESS = { whileTap: { scale: 0.96 }, transition: { type: "spring" as const, stiffness: 500, damping: 30 } };

export function Hud({ onEdit }: { onEdit?: (folder: string) => Promise<void> }) {
  const { displays, mics, sel, setSel } = useDevices();
  const [camId, setCamId] = useState<string | null>(null); const [camOn, setCamOn] = useState(true);
  // `useRecordingFlow` needs a stable `camStream` getter, but `cam`'s own `enabled` (below) needs
  // `saving`, which only exists once `useRecordingFlow` returns - broken via this ref instead of a
  // circular reference; `webcam.start` only ever calls it later, once `cam` has set the real getter.
  const camStreamRef = useRef<() => MediaStream | null>(() => null);
  const [menu, setMenu] = useState<string | null>(null);
  const [micOn, setMicOn] = useState(true); const [sysOn, setSysOn] = useState(false); const [gameMode, setGameMode] = useState(false);
  const [exporting, setExporting] = useState(false); const [pct, setPct] = useState(0);
  const [exportErr, setExportErr] = useState<string | null>(null);
  const [panel, setPanel] = useState<"settings" | "preferences" | null>(null);
  const [barShown, setBarShown] = useState(true);
  const lastFolder = useRef<string>("");
  const themeRef = useRef<{ theme: ThemeMode; accent: [number, number, number] }>({ theme: "light", accent: [239, 68, 68] });
  // Task 39's feel knob - gates whether the titlebar mark flows/pulses at all. Real React state
  // (unlike theme/accent, applied imperatively via CSS vars) because it's a prop TcursorMark reads.
  const [animatedBrand, setAnimatedBrand] = useState(true);
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

  const BOX_W = 360, BOX_H = 440; // 16px body top-padding + 420 box + a little slack
  // The box<->bar window resize is driven by openSettings and the settings exit
  // (restoreBar via onExitComplete) so the spring-out is never clipped; this hook only tracks
  // the dropdown height while the bar is showing, and fixes the cold-launch centering bug (L1) -
  // see its own doc comment. `saving` wins over `recording` (mode, item 3 - see useHudWindowSize.md).
  useHudWindowSize(menu, barShown, saving ? "saving" : recording ? "recording" : "idle");

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
  const camOpts = cameras.length ? cameras.map((c) => ({ id: c.id, label: cleanDeviceLabel(c.label) })) : [{ id: "", label: "Camera" }];
  const banner = err ?? exportErr;

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
            {/* Carries its own subject - a recording failure, a degraded-but-running take, an
                OS-ended one, or an export failure - so this one slot covers all of them. */}
            {banner && <span className="banner" title={banner}>⚠ {banner}</span>}
            <span className="winctrls">
              {!recording && !exporting && (<>
                <button className="winbtn" title="Open Project" onClick={openExistingProject} disabled={saving}><FolderOpen /></button>
                <button className="winbtn" title="Preferences" onClick={() => openPanel("preferences")} disabled={saving}><Palette /></button>
                <button className="winbtn gear" title="Settings" onClick={() => openPanel("settings")} disabled={saving}><Gear /></button>
              </>)}
              <button className="winbtn" title="Minimize" onClick={() => win.minimize()}><MinIcon /></button>
              <button className="winbtn close" title="Close" onClick={handleClose}><CloseIcon /></button>
            </span>
          </div>

          <div className="row">
            <div className="grip" data-tauri-drag-region><Grip /></div>

            {/* Item 4: hidden entirely (not dimmed) while saving - `cam`'s `enabled` gate above already released the stream. */}
            {!saving && <CamTile camRef={cam.ref} camOn={camOn} camLive={cam.on} />}

            {exporting ? (
              <span className="exporting">Exporting… {pct}%</span>
            ) : saving ? (
              <span className="exporting saving">Saving… {savePct}%</span>
            ) : !recording ? (
              <>
                <div className="pickers">
                  <Dropdown icon={<Camera />} value={camId ?? cameras[0]?.id ?? ""} options={camOpts}
                    open={menu === "cam"} onToggle={() => tg("cam")} onPick={(id) => { setCamId(id || null); setMenu(null); }} />
                  <TargetPicker targets={displays} value={sel.displayId ?? ""}
                    open={menu === "screen"} onToggle={() => tg("screen")} onPick={(id) => { setSel({ ...sel, displayId: id }); setMenu(null); }} />
                  <Dropdown icon={<Mic />} value={sel.micId ?? ""} options={mics.map((m) => ({ id: m.id, label: cleanDeviceLabel(m.label) }))}
                    open={menu === "mic"} onToggle={() => tg("mic")} onPick={(id) => { setSel({ ...sel, micId: id }); setMenu(null); }} />
                </div>
                <div className="toggle-group">
                  <motion.button className={`toggle ${camOn ? "on" : ""}`} title={camOn ? "Camera on" : "Camera off"} onClick={() => setCamOn(v => !v)} {...TOGGLE_PRESS}>{camOn ? <Camera /> : <CameraOff />}</motion.button>
                  <motion.button className={`toggle ${micOn ? "on" : ""}`} title={micOn ? "Microphone on" : "Microphone off"} onClick={() => setMicOn(v => !v)} {...TOGGLE_PRESS}>{micOn ? <Mic /> : <MicOff />}</motion.button>
                  <motion.button className={`toggle ${sysOn ? "on" : ""}`} title={sysOn ? "System audio on" : "System audio off"} onClick={() => setSysOn(v => !v)} {...TOGGLE_PRESS}>{sysOn ? <Speaker /> : <SpeakerOff />}</motion.button>
                  <motion.button className={`toggle ${gameMode ? "on" : ""}`} title={gameMode ? "Compatibility encoder (on): legacy CPU recording" : "Compatibility encoder: switch on only if GPU recording has issues"} onClick={() => setGameMode(v => !v)} {...TOGGLE_PRESS}><Gamepad /></motion.button>
                </div>
              </>
            ) : (
              <RecMeter micOn={micOn} live={audio.live} read={audio.read} />
            )}

            <div className="spacer" />

            <RecordingControls recording={recording} paused={paused} elapsed={elapsed} saving={saving}
              exporting={exporting} toggle={toggle} togglePause={togglePause} />
          </div>
        </>
      )}
    </div>
    </MotionConfig>
  );
}
