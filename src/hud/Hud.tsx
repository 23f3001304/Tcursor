import { useRef, useState } from "react";
import { MotionConfig } from "motion/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useDevices } from "./hooks/useDevices";
import { useHudWindowSize, takeWidth } from "./hooks/useHudWindowSize";
import { useRecordingTimer } from "./hooks/useRecordingTimer";
import { useWebcamPreview } from "./hooks/useWebcamPreview";
import { useCameraDevices } from "./hooks/useCameraDevices";
import { useAudioLevels } from "./hooks/useAudioLevels";
import { useRecordingFlow } from "./hooks/useRecordingFlow";
import { useSourceSwitch } from "./hooks/useSourceSwitch";
import { useExportProgress } from "./hooks/useExportProgress";
import { useHudTheme } from "./hooks/useHudTheme";
import { useHudChrome } from "./hooks/useHudChrome";
import { IdleCard } from "./components/IdleCard";
import type { Toggles } from "./components/SourceToggles";
import { TakeBar } from "./components/TakeBar";
import { SourcesSheet } from "./components/SourcesSheet";
import { StateSwap } from "./components/StateSwap";
import { cleanDeviceLabel } from "./devices/selectDevices";
import { openProject } from "../shared/ipc";
import { useWebcamRecorder } from "./hooks/useWebcamRecorder";
import { Settings } from "./settings/SettingsPanel";
import { Preferences } from "./preferences/Preferences";

export function Hud({ onEdit }: { onEdit?: (folder: string) => Promise<void> }) {
  const { displays, mics, sel, setSel } = useDevices();
  const [camId, setCamId] = useState<string | null>(null);
  const [camOn, setCamOn] = useState(true);
  const camStreamRef = useRef<() => MediaStream | null>(() => null);
  const [micOn, setMicOn] = useState(true);
  const [sysOn, setSysOn] = useState(false);
  const [gameMode, setGameMode] = useState(false);
  const [takeShown, setTakeShown] = useState(false);
  const lastFolder = useRef<string>("");
  const onUi = useHudTheme();
  const { exporting, pct, exportErr } = useExportProgress(lastFolder);
  const webcam = useWebcamRecorder();
  const { recording, paused, saving, savePct, err, toggle, togglePause, stopForClose } = useRecordingFlow({
    micOn,
    micId: sel.micId,
    displayId: sel.displayId,
    sysOn,
    gameMode,
    camOn,
    camStream: () => camStreamRef.current(),
    webcam,
    onEdit,
    lastFolderRef: lastFolder,
  });
  const chrome = useHudChrome(recording);
  const cam = useWebcamPreview(camId, camOn && !saving);
  camStreamRef.current = cam.stream;
  const cameras = useCameraDevices(cam.on ? 1 : 0);
  const elapsed = useRecordingTimer(recording, paused);
  const audio = useAudioLevels(recording && !paused && (micOn || sysOn));
  const win = getCurrentWindow();

  const take = recording || saving;
  const src = useSourceSwitch({
    recording,
    camOn,
    micOn,
    camId,
    camStream: () => camStreamRef.current(),
    webcam,
    setCamId,
    setMicId: (id) => setSel((s) => ({ ...s, micId: id })),
    setDisplayId: (id) => setSel((s) => ({ ...s, displayId: id })),
  });
  const warn = err ?? src.err;
  const sourcesOpen = chrome.sources && !saving;
  const pill = { cam: camOn, meter: micOn || sysOn, warn: warn != null, sources: sourcesOpen };
  useHudWindowSize(chrome.menu, takeShown ? (saving ? "saving" : "recording") : "idle", pill);

  async function handleClose() {
    await stopForClose();
    win.close();
  }

  async function openExistingProject() {
    try {
      await onEdit?.(await openProject());
    } catch {}
  }
  const setters = { camOn: setCamOn, micOn: setMicOn, sysOn: setSysOn, gameMode: setGameMode };
  const flip = (k: keyof Toggles) => setters[k]((v) => !v);
  const camOpts = cameras.length
    ? cameras.map((c) => ({ id: c.id, label: cleanDeviceLabel(c.label) }))
    : [{ id: "", label: "Camera" }];
  const micOpts = mics.map((m) => ({ id: m.id, label: cleanDeviceLabel(m.label) }));
  const banner = warn ?? exportErr;
  const panelBody =
    chrome.panel === "preferences" ? (
      <Preferences onClose={chrome.closePanel} onUiChange={onUi} />
    ) : chrome.panel === "settings" ? (
      <Settings onClose={chrome.closePanel} />
    ) : null;

  return (
    <MotionConfig reducedMotion="user">
      <div
        className={`hud${takeShown ? " as-take" : ""}${sourcesOpen ? " has-sources" : ""}`}
        style={takeShown ? { width: takeWidth(pill) } : undefined}
      >
        <StateSwap
          take={take}
          onSettled={setTakeShown}
          pill={
            <>
              <TakeBar
                paused={paused}
                saving={saving}
                savePct={savePct}
                elapsed={elapsed}
                err={warn}
                micOn={micOn}
                sysOn={sysOn}
                live={audio.live}
                read={audio.read}
                camRef={cam.ref}
                camOn={camOn}
                camLive={cam.on}
                toggle={toggle}
                togglePause={togglePause}
                sources={sourcesOpen}
                onSources={() => chrome.openSources(src.clearErr)}
              />
              {sourcesOpen && (
                <SourcesSheet
                  targets={displays}
                  displayId={sel.displayId ?? ""}
                  onTarget={(id) => chrome.picked(() => void src.switchDisplay(id))}
                  cameras={camOpts}
                  camId={camId ?? cameras[0]?.id ?? ""}
                  onCam={(id) => chrome.picked(() => void src.switchCamera(id || null))}
                  mics={micOpts}
                  micId={sel.micId ?? ""}
                  onMic={(id) => chrome.picked(() => void src.switchMic(id))}
                  menu={chrome.menu}
                  onMenu={chrome.toggleMenu}
                  sheet={chrome.sheet}
                  onSheet={chrome.setSheet}
                />
              )}
            </>
          }
          idle={
            <IdleCard
              banner={banner}
              exporting={exporting}
              pct={pct}
              onOpenProject={openExistingProject}
              onPreferences={() => chrome.openPanel("preferences")}
              onSettings={() => chrome.openPanel("settings")}
              onMinimize={() => void win.minimize()}
              onClose={() => void handleClose()}
              camRef={cam.ref}
              camLive={cam.on}
              cameras={camOpts}
              camId={camId ?? cameras[0]?.id ?? ""}
              onCam={(id) => {
                setCamId(id || null);
                chrome.closeMenu();
              }}
              targets={displays}
              displayId={sel.displayId ?? ""}
              onTarget={(id) => setSel({ ...sel, displayId: id })}
              mics={micOpts}
              micId={sel.micId ?? ""}
              onMic={(id) => {
                setSel({ ...sel, micId: id });
                chrome.closeMenu();
              }}
              menu={chrome.menu}
              onMenu={chrome.toggleMenu}
              sheet={chrome.sheet}
              onSheet={chrome.setSheet}
              panel={chrome.panel}
              panelBody={panelBody}
              toggles={{ camOn, micOn, sysOn, gameMode }}
              onToggle={flip}
              onRecord={toggle}
            />
          }
        />
      </div>
    </MotionConfig>
  );
}
