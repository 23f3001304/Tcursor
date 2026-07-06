import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getEdit, setCapturable, cameraTrack, previewLayout, previewLayouts, clickTrack, previewBg, cursorSprites, cursorKinds, ensureThumbs, ensureWaveform, ensurePreviewAudio, ensureProxy, fileSrc } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, LayoutPresets } from "../../lib/ipc";

// All the editor's preview/timeline data fetching, split out of Editor so the component
// itself only holds render + mutation logic. See docs/api/src/editor/Editor.md for why each
// fetch is keyed on [folder], [folder, rev], or [folder, quality].
export function useEditorData(folder: string, rev: number, quality: number) {
  const [doc, setDoc] = useState<EditDoc | null>(null);
  const [track, setTrack] = useState<CamSample[]>([]);
  const [layout, setLayout] = useState<PreviewLayout | null>(null);
  const [layoutPresets, setLayoutPresets] = useState<LayoutPresets | null>(null);
  const [clicks, setClicks] = useState<ClickSample[]>([]);
  const [bgUrl, setBgUrl] = useState("");
  const [cursorSpr, setCursorSpr] = useState<CursorSpriteDto[]>([]);
  const [cursorKnd, setCursorKnd] = useState<CursorKindSample[]>([]);
  const [thumbs, setThumbs] = useState<string[]>([]);
  const [waves, setWaves] = useState<{ system: string; mic: string }>({ system: "", mic: "" });
  const [audioUrl, setAudioUrl] = useState("");
  const [srcUrl, setSrcUrl] = useState("");
  const [playing, setPlaying] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);

  useEffect(() => { getEdit(folder).then(setDoc).catch(() => {}); void setCapturable(true); }, [folder]);
  // The exact zoom curve, refetched whenever the doc changes (instant: pure math, cached).
  useEffect(() => { cameraTrack(folder).then(setTrack).catch(() => {}); }, [folder, rev]);
  // The exact screen/webcam framing (pad, radius, PiP rect) so the canvas matches the export.
  useEffect(() => { previewLayout(folder).then(setLayout).catch(() => {}); }, [folder, rev]);
  // All 5 layout presets' panel rects, so the preview can cross-fade across layout segments
  // itself (layoutAt) instead of only ever showing the single static layout above.
  useEffect(() => { previewLayouts(folder).then(setLayoutPresets).catch(() => {}); }, [folder, rev]);
  // Click ripples + the exact export background - neither changes with edits, so fetch once
  // per folder (refetching on every edit was part of the add-effect lag).
  useEffect(() => { clickTrack(folder).then(setClicks).catch(() => {}); }, [folder]);
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

  return {
    doc, setDoc, track, layout, layoutPresets, clicks, bgUrl, cursorSpr, cursorKnd,
    thumbs, waves, audioUrl, srcUrl, playing, setPlaying, exporting, setExporting, pct, setPct,
  };
}
