import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getEdit, setCapturable, cameraTrack, previewLayout, previewLayouts, clickTrack, previewBg, cursorSprites, cursorKinds, ensureThumbs, ensureWaveform, ensurePreviewAudio, ensureProxy, fileSrc, getProjectManifest, DEFAULT_PROXY_HEIGHT } from "../../lib/ipc";
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
  const [exportDone, setExportDone] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  useEffect(() => { getEdit(folder).then(setDoc).catch(() => {}); void setCapturable(true); }, [folder]);
  // Was this project already preprocessed at save time (see `preprocess_project`)? If so, the
  // proxy fetch below can point straight at the known path instead of round-tripping through
  // `ensureProxy` - `preprocessed` only ever flips true on the Rust side after that exact call
  // already succeeded once. Defaults to false (the pre-existing lazy path) until the manifest
  // read resolves, so a legacy project with no manifest at all is unaffected.
  const [preprocessed, setPreprocessed] = useState(false);
  // `manifestReady` gates the proxy source below until we KNOW whether this project is
  // preprocessed - otherwise the effect runs once with the default `preprocessed=false`, loads
  // the heavy raw 4K `video.mp4`, and only swaps to the light proxy after the manifest read
  // resolves (the "first load feels laggy" flash). The manifest read is a tiny file read, so the
  // brief wait is imperceptible and avoids decoding a 4K frame the preprocessed project never needs.
  const [manifestReady, setManifestReady] = useState(false);
  useEffect(() => {
    setManifestReady(false);
    getProjectManifest(folder)
      .then((m) => { setPreprocessed(m.preprocessed); setManifestReady(true); })
      .catch(() => { setPreprocessed(false); setManifestReady(true); });
  }, [folder]);
  // The exact zoom curve, refetched whenever the doc changes (instant: pure math, cached).
  useEffect(() => { cameraTrack(folder).then(setTrack).catch(() => {}); }, [folder, rev]);
  // The exact screen/webcam framing (pad, radius, PiP rect) so the canvas matches the export.
  useEffect(() => { previewLayout(folder).then(setLayout).catch(() => {}); }, [folder, rev]);
  // All 5 layout presets' panel rects, so the preview can cross-fade across layout segments
  // itself (layoutAt) instead of only ever showing the single static layout above.
  useEffect(() => { previewLayouts(folder).then(setLayoutPresets).catch(() => {}); }, [folder, rev]);
  // Click ripples never change with edits, so fetch once per folder (refetching on every edit
  // was part of the add-effect lag).
  useEffect(() => { clickTrack(folder).then(setClicks).catch(() => {}); }, [folder]);
  // The export background: refetch only when the doc's OWN background settings change (same
  // "don't refetch on every unrelated edit" reasoning as cursorSprites below) - a background
  // panel edit is the only kind of change that can actually alter what `preview_bg` returns.
  useEffect(() => {
    previewBg(folder).then(setBgUrl).catch(() => {});
  }, [folder, JSON.stringify(doc?.settings.background)]);
  // Cursor type track never changes with edits, so fetch once per folder.
  useEffect(() => { cursorKinds(folder).then(setCursorKnd).catch(() => {}); }, [folder]);
  // Cursor sprite pack: refetch when the doc's selected pack changes (picking a different pack,
  // or importing one, in CursorPanel) - NOT on generic `rev` bumps, so unrelated edits don't
  // re-decode sprites (same "don't refetch on every edit" reasoning as clickTrack/previewBg above).
  useEffect(() => {
    if (!doc) return;
    cursorSprites(folder).then(setCursorSpr).catch(() => {});
  }, [folder, doc?.settings.cursor.pack]);
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
  const proxyReadyRef = useRef(false);
  const lastFolderRef = useRef(folder);
  useEffect(() => {
    setPlaying(false);
    if (lastFolderRef.current !== folder) { lastFolderRef.current = folder; proxyReadyRef.current = false; }
    if (!manifestReady) return; // wait until we know `preprocessed`, so a preprocessed project never loads raw 4K first
    // Raw fast-path ONLY for a not-yet-preprocessed project with no proxy yet - show something while
    // the proxy transcodes. A preprocessed project skips straight to its proxy below (no 4K load).
    // On a quality SWITCH the current proxy is already showing, so keep it (its re-timed timeline/
    // duration match) instead of flashing raw video.mp4, which re-times differently and jumps the
    // playhead ("changing quality changes preview time").
    if (!proxyReadyRef.current && !preprocessed) setSrcUrl(fileSrc(`${folder}\\video.mp4`));
    // Skip the lazy transcode entirely at the DEFAULT quality on an already-preprocessed project -
    // that exact proxy is guaranteed to already be on disk (see `preprocessed` above), so there is
    // nothing to generate. Any OTHER quality (the in-editor quality toggle) still needs its own
    // transcode and falls through to `ensureProxy` below, same as a legacy/un-preprocessed project.
    if (preprocessed && quality === DEFAULT_PROXY_HEIGHT) {
      setSrcUrl(fileSrc(`${folder}\\preview_${quality}_rt.mp4`));
      proxyReadyRef.current = true;
      return;
    }
    ensureProxy(folder, quality).then((p) => { setSrcUrl(fileSrc(p)); proxyReadyRef.current = true; }).catch(() => {});
  }, [folder, quality, preprocessed]);

  useEffect(() => {
    const subs = [
      listen<number>("export-progress", (e) => setPct(e.payload)),
      listen("export-done", () => { setExporting(false); setExportDone(true); }),
      listen<string>("export-error", (e) => { setExporting(false); setExportError(e.payload); }),
    ];
    return () => { subs.forEach((s) => s.then((f) => f())); };
  }, []);

  return {
    doc, setDoc, track, layout, layoutPresets, clicks, bgUrl, cursorSpr, cursorKnd,
    thumbs, waves, audioUrl, srcUrl, playing, setPlaying, exporting, setExporting, pct, setPct,
    exportDone, setExportDone, exportError, setExportError,
  };
}
