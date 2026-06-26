import { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";
import { useDevices } from "./useDevices";
import { useRecordingTimer } from "./useRecordingTimer";
import { useWebcamPreview } from "./useWebcamPreview";
import { useCameraDevices } from "./useCameraDevices";
import { useMicWaveform } from "./useMicWaveform";
import { formatTimer } from "./formatTimer";
import { Dropdown } from "./Dropdown";
import { Grip, Monitor, Mic, MicOff, Speaker, SpeakerOff, Camera, MinIcon, CloseIcon } from "./icons";
import { startRecording, stopRecording, pauseRecording, resumeRecording, saveWebcam, exportProject } from "../lib/ipc";
import { useWebcamRecorder } from "./useWebcamRecorder";

const WIDTH = 860;

export function Hud() {
  const { displays, mics, sel, setSel } = useDevices();
  const [camId, setCamId] = useState<string | null>(null);
  const [camOn, setCamOn] = useState(true);
  const cam = useWebcamPreview(camId, camOn);
  const cameras = useCameraDevices(cam.on ? 1 : 0);
  const [recording, setRecording] = useState(false);
  const [paused, setPaused] = useState(false);
  const [menu, setMenu] = useState<string | null>(null);
  const [micOn, setMicOn] = useState(true);
  const [sysOn, setSysOn] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const lastFolder = useRef<string>("");
  const webcam = useWebcamRecorder();
  const elapsed = useRecordingTimer(recording, paused);
  const levels = useMicWaveform(recording && !paused);
  const win = getCurrentWindow();

  useEffect(() => { win.setSize(new LogicalSize(WIDTH, menu ? 430 : 132)); }, [menu]);

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(listen<number>("export-progress", e => setPct(e.payload)));
    unsubs.push(listen<string>("export-done", e => {
      setExporting(false);
      revealItemInDir(`${e.payload}\\final.mp4`).catch(() => {});
    }));
    unsubs.push(listen<string>("export-error", () => {
      setExporting(false);
      revealItemInDir(`${lastFolder.current}\\video.mp4`).catch(() => {});
    }));
    return () => { unsubs.forEach(u => u.then(f => f())); };
  }, []);

  async function toggle() {
    if (!recording) {
      await startRecording(`rec-${Date.now()}`, micOn ? sel.micId : null, sysOn);
      if (camOn) webcam.start(cam.ref.current?.srcObject as MediaStream | null);
      setRecording(true);
    } else {
      const res = await stopRecording();
      const bytes = await webcam.stop();
      if (bytes) await saveWebcam(res.folder, bytes);
      lastFolder.current = res.folder;
      setRecording(false);
      setPaused(false);
      setPct(0);
      setExporting(true);
      await exportProject(res.folder);
    }
  }
  async function togglePause() {
    if (paused) { await resumeRecording(); setPaused(false); }
    else { await pauseRecording(); setPaused(true); }
  }
  const tg = (id: string) => setMenu((m) => (m === id ? null : id));
  const camOpts = cameras.length ? cameras.map((c) => ({ id: c.id, label: c.label })) : [{ id: "", label: "Camera" }];

  return (
    <div className="hud">
      <div className="titlebar" data-tauri-drag-region>
        <span className="brand">CursorZoom</span>
        <span className="winctrls">
          <button className="winbtn" title="Minimize" onClick={() => win.minimize()}><MinIcon /></button>
          <button className="winbtn close" title="Close" onClick={() => win.close()}><CloseIcon /></button>
        </span>
      </div>

      <div className="row">
        <div className="grip" data-tauri-drag-region><Grip /></div>

        <button className="camtoggle" title={recording ? "Camera locked while recording" : camOn ? "Turn camera off" : "Turn camera on"} onClick={() => { if (!recording) setCamOn((v) => !v); }}>
          <video ref={cam.ref} className={`cam ${camOn && cam.on ? "" : "off"}`} autoPlay muted playsInline />
          {!(camOn && cam.on) && <span className="camoff"><Camera /></span>}
        </button>

        {exporting ? (
          <span className="exporting">Exporting… {pct}%</span>
        ) : !recording ? (
          <>
            <Dropdown icon={<Camera />} value={camId ?? cameras[0]?.id ?? ""} options={camOpts}
              open={menu === "cam"} onToggle={() => tg("cam")} onPick={(id) => { setCamId(id || null); setMenu(null); }} />
            <div className="divider" />
            <Dropdown icon={<Monitor />} value={String(sel.displayId ?? "")} options={displays.map((d) => ({ id: String(d.id), label: d.label }))}
              open={menu === "screen"} onToggle={() => tg("screen")} onPick={(id) => { setSel({ ...sel, displayId: Number(id) }); setMenu(null); }} />
            <Dropdown icon={<Mic />} value={sel.micId ?? ""} options={mics.map((m) => ({ id: m.id, label: m.label }))}
              open={menu === "mic"} onToggle={() => tg("mic")} onPick={(id) => { setSel({ ...sel, micId: id }); setMenu(null); }} />
            <button className={`toggle ${micOn ? "on" : ""}`} title={micOn ? "Microphone on" : "Microphone off"} onClick={() => setMicOn(v => !v)}>{micOn ? <Mic /> : <MicOff />}</button>
            <button className={`toggle ${sysOn ? "on" : ""}`} title={sysOn ? "System audio on" : "System audio off"} onClick={() => setSysOn(v => !v)}>{sysOn ? <Speaker /> : <SpeakerOff />}</button>
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
        <button className={`btn rec ${recording ? "is-rec" : ""}`} onClick={toggle} disabled={exporting}>
          <span className="dot" />{recording ? "Stop" : "Record"}
        </button>
      </div>
    </div>
  );
}
