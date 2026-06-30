import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getEdit, aiAutoedit, exportProject, applyEditOp, setCapturable, cameraTrack, previewLayout, clickTrack, spotlightHolds, previewBg, cursorSprites, cursorKinds, ensureThumbs, ensureWaveform, ensurePreviewAudio, ensureProxy, fileSrc } from "../lib/ipc";
import type { EditDoc, EditOp } from "../lib/edit";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, HoldSpan } from "../lib/ipc";
import { TopBar } from "./TopBar";
import { ResizeEdges } from "./ResizeEdges";
import { Rail, type Tab } from "./Rail";
import { AiPanel } from "./AiPanel";
import { ZoomInspector } from "./ZoomInspector";
import { EffectInspector } from "./EffectInspector";
import { Stage } from "./Stage";
import { Transport } from "./Transport";
import { Timeline } from "./Timeline";
import "./editor.css";

/** The post-record editor. The preview plays the recording natively (Stage) and applies
 *  the exact camera curve (camera_track) as a transform - smooth 60fps. Edits go through
 *  the Edit API; `rev` bumps so the camera curve refetches and reflects them. */
export function Editor({ folder, onClose }: { folder: string; onClose: () => void }) {
  const [doc, setDoc] = useState<EditDoc | null>(null);
  const [track, setTrack] = useState<CamSample[]>([]);
  const [layout, setLayout] = useState<PreviewLayout | null>(null);
  const [clicks, setClicks] = useState<ClickSample[]>([]);
  const [spotHolds, setSpotHolds] = useState<HoldSpan[]>([]);
  const [bgUrl, setBgUrl] = useState("");
  const [cursorSpr, setCursorSpr] = useState<CursorSpriteDto[]>([]);
  const [cursorKnd, setCursorKnd] = useState<CursorKindSample[]>([]);
  const [thumbs, setThumbs] = useState<string[]>([]);
  const [waves, setWaves] = useState<{ system: string; mic: string }>({ system: "", mic: "" });
  const [audioUrl, setAudioUrl] = useState("");
  const [muted, setMuted] = useState(false);
  const [timeMs, setTimeMs] = useState(0);
  const [rev, setRev] = useState(0);
  const [tab, setTab] = useState<Tab>("ai");
  const [sel, setSel] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [running, setRunning] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const [vidDurMs, setVidDurMs] = useState(0);
  const [quality, setQuality] = useState(720);
  const [srcUrl, setSrcUrl] = useState("");

  // The <video>'s own duration is the source of truth (the encoded length can differ
  // from the capture-timestamp span the seed used), so the playhead reaches the true end.
  const dur = vidDurMs > 0 ? vidDurMs : (doc?.trim.out_ms ?? 0);
  const proj = folder.split(/[\\/]/).pop() ?? "project";

  useEffect(() => { getEdit(folder).then(setDoc).catch(() => {}); void setCapturable(true); }, [folder]);
  // The exact zoom curve, refetched whenever the doc changes (instant: pure math, cached).
  useEffect(() => { cameraTrack(folder).then(setTrack).catch(() => {}); }, [folder, rev]);
  // The exact screen/webcam framing (pad, radius, PiP rect) so the canvas matches the export.
  useEffect(() => { previewLayout(folder).then(setLayout).catch(() => {}); }, [folder, rev]);
  // Click ripples + the exact export background - neither changes with edits, so fetch once
  // per folder (refetching on every edit was part of the add-effect lag).
  useEffect(() => { clickTrack(folder).then(setClicks).catch(() => {}); }, [folder]);
  // Recorded spotlight holds (hotkey-held during capture) - immutable, so fetch once per folder.
  useEffect(() => { spotlightHolds(folder).then(setSpotHolds).catch(() => {}); }, [folder]);
  useEffect(() => { previewBg(folder).then(setBgUrl).catch(() => {}); }, [folder]);
  // Cursor sprite pack + type track (don't change with edits) so the preview cursor matches export.
  useEffect(() => { cursorSprites(folder).then(setCursorSpr).catch(() => {}); cursorKinds(folder).then(setCursorKnd).catch(() => {}); }, [folder]);
  // Timeline media: filmstrip thumbnails, the system/mic waveform images, and the mixed audio.
  useEffect(() => {
    ensureThumbs(folder, 16).then((p) => setThumbs(p.map(fileSrc))).catch(() => {});
    Promise.all([ensureWaveform(folder, "system"), ensureWaveform(folder, "mic")])
      .then(([s, m]) => setWaves({ system: s ? fileSrc(s) : "", mic: m ? fileSrc(m) : "" })).catch(() => {});
    ensurePreviewAudio(folder).then((p) => setAudioUrl(p ? fileSrc(p) : "")).catch(() => {});
  }, [folder]);
  // Transcode (once, cached) a low-res preview proxy and play that instead of the raw 4K.
  // Fast load: show the raw capture instantly (no transcode wait), then hot-swap to the light
  // proxy once it has transcoded (for smoother scrubbing). Pause first so the src remount can't
  // restart playback from 0.
  useEffect(() => {
    setPlaying(false);
    setSrcUrl(fileSrc(`${folder}\\video.mp4`));
    ensureProxy(folder, quality).then((p) => setSrcUrl(fileSrc(p))).catch(() => {});
  }, [folder, quality]);

  useEffect(() => {
    const subs = [
      listen<number>("export-progress", (e) => setPct(e.payload)),
      listen("export-done", () => { setExporting(false); setPct(0); }),
      listen("export-error", () => { setExporting(false); setPct(0); }),
    ];
    return () => { subs.forEach((s) => s.then((f) => f())); };
  }, []);

  // Playback is driven by the native <video> (Stage reports its time here); stop at the end.
  const onTime = (ms: number) => { setTimeMs(ms); if (dur > 0 && ms >= dur) setPlaying(false); };

  // Persist an edit op, swap in the returned doc, and bump rev (refetches the camera curve).
  const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
    try {
      const d = await applyEditOp(folder, op); setDoc(d);
      // Effect edits don't change the camera/layout, so skip the rev bump (and its renderer
      // rebuild + refetch) - the preview reflects them straight from the returned doc. This is
      // why adding/dragging a spotlight is instant instead of laggy.
      if (!op.op.endsWith("_effect")) setRev((r) => r + 1);
      return d;
    } catch { return null; }
  };
  const addZoom = async () => {
    const d = await applyOp({ op: "add_zoom", at_ms: Math.round(timeMs), dur_ms: 2000 });
    if (d && d.zooms.length) setSel(d.zooms[d.zooms.length - 1].id);
  };
  const addSpotlight = async () => {
    const d = await applyOp({ op: "add_effect", kind: "spotlight", start_ms: Math.round(timeMs), end_ms: Math.round(timeMs) + 2000 });
    if (d && d.effects.length) setSel(d.effects[d.effects.length - 1].id);
  };
  // Click the preview to add a zoom at the playhead focused on the clicked point (Fixed target).
  const zoomAt = async (x: number, y: number) => {
    setPlaying(false);
    const d = await applyOp({ op: "add_zoom_full", at_ms: Math.round(timeMs), dur_ms: 2000, scale: 2.5 });
    if (d && d.zooms.length) {
      const id = d.zooms[d.zooms.length - 1].id;
      await applyOp({ op: "update_zoom", id, target: { fixed: { x, y } } });
      setSel(id);
    }
  };

  const onExport = () => { setExporting(true); setPct(0); void exportProject(folder); };
  const onRun = async () => {
    setRunning(true);
    try { setDoc(await aiAutoedit(folder)); setRev((r) => r + 1); } catch { /* Ollama down */ } finally { setRunning(false); }
  };

  if (!doc) return <div className="editor"><div className="e-stage-empty" style={{ margin: "auto" }}>Loading edit...</div></div>;

  const selZoom = doc.zooms.find((z) => z.id === sel) ?? null;
  const selEffect = doc.effects.find((e) => e.id === sel) ?? null;

  return (
    <div className="editor">
      <ResizeEdges />
      <TopBar proj={proj} exporting={exporting} pct={pct} onExport={onExport} onClose={onClose} />
      <div className="e-body">
        <Rail tab={tab} onTab={(t) => { setSel(null); setTab(t); }} />
        {selZoom ? (
          <ZoomInspector zoom={selZoom} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
        ) : selEffect ? (
          <EffectInspector effect={selEffect} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
        ) : tab === "ai" ? (
          <AiPanel running={running} onRun={onRun} />
        ) : (
          <div className="e-panel">
            <h2 style={{ textTransform: "capitalize" }}>{tab}</h2>
            <p className="e-lede">{tab} settings land here next.</p>
          </div>
        )}
        <Stage src={srcUrl} webcamSrc={fileSrc(`${folder}\\webcam.webm`)} track={track} layout={layout} clicks={clicks} bgUrl={bgUrl} cursorSprites={cursorSpr} cursorKinds={cursorKnd} cursor={doc.settings.cursor} effects={doc.effects} spotlightHolds={spotHolds} clickfx={doc.settings.clickfx} audioSrc={audioUrl} muted={muted} timeMs={timeMs} playing={playing} onTime={onTime} onDuration={setVidDurMs} onZoomAt={zoomAt} />
      </div>
      <Transport timeMs={timeMs} dur={dur} playing={playing} onPlay={() => setPlaying((p) => !p)} onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }} onAddZoom={addZoom} onAddSpotlight={addSpotlight}
        quality={quality} onQuality={() => setQuality((q) => (q === 480 ? 720 : q === 720 ? 1080 : 480))}
        muted={muted} onMute={() => setMuted((m) => !m)} />
      <Timeline doc={doc} timeMs={timeMs} dur={dur} onSeek={(ms) => { setPlaying(false); setTimeMs(ms); }} sel={sel} onSel={setSel} onApply={applyOp} thumbs={thumbs} waves={waves} />
    </div>
  );
}
