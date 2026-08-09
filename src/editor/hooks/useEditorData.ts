import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getEdit, setCapturable, cameraTrack, previewLayout, previewLayouts, clickTrack, previewBg, cursorSprites, cursorKinds, osCursorInVideo, ensureThumbs, ensureWaveform, ensurePreviewAudio, ensureProxy, fileSrc, getProjectManifest, DEFAULT_PROXY_HEIGHT } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import { planProxySrc } from "./editorData";

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
  const [osCursor, setOsCursor] = useState(true);
  const [thumbs, setThumbs] = useState<string[]>([]);
  const [waves, setWaves] = useState<{ system: string; mic: string }>({ system: "", mic: "" });
  const [audioUrl, setAudioUrl] = useState("");
  const [srcUrl, setSrcUrl] = useState("");
  const [playing, setPlaying] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [pct, setPct] = useState(0);
  const [exportDone, setExportDone] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportPath, setExportPath] = useState(""); // the finished export's own file path (export-done's payload), for "Show in folder"

  useEffect(() => {
    let live = true;
    getEdit(folder).then((d) => { if (live) setDoc(d); }).catch(() => {});
    void setCapturable(true);
    return () => { live = false; };
  }, [folder]);
  // Whether this project was already preprocessed at save time (see `preprocess_project`), and
  // whether that read has resolved at all - kept as ONE state object (not two separate `useState`
  // pairs) so the proxy effect below can key its dependency array on `manifest` alone and always
  // re-run when EITHER field changes. The old two-state shape keyed the proxy effect's deps on
  // `preprocessed` alone: a legacy/un-preprocessed project (or a corrupt/failed manifest read)
  // resolves `preprocessed: false` - the SAME value as the pre-read default - so only `ready`
  // actually flips, which wasn't in the deps array, so the effect never re-ran and `srcUrl` stayed
  // "" forever (permanent spinner). Defaults to `{ ready: false, preprocessed: false }`.
  const [manifest, setManifest] = useState({ ready: false, preprocessed: false });
  useEffect(() => {
    let live = true;
    setManifest({ ready: false, preprocessed: false });
    getProjectManifest(folder)
      .then((m) => { if (live) setManifest({ ready: true, preprocessed: m.preprocessed }); })
      .catch(() => { if (live) setManifest({ ready: true, preprocessed: false }); });
    return () => { live = false; };
  }, [folder]);
  // The exact zoom curve, refetched whenever the doc changes (instant: pure math, cached).
  useEffect(() => {
    let live = true;
    cameraTrack(folder).then((d) => { if (live) setTrack(d); }).catch(() => {});
    return () => { live = false; };
  }, [folder, rev]);
  // The exact screen/webcam framing (pad, radius, PiP rect) so the canvas matches the export.
  useEffect(() => {
    let live = true;
    previewLayout(folder).then((d) => { if (live) setLayout(d); }).catch(() => {});
    return () => { live = false; };
  }, [folder, rev]);
  // All 5 layout presets' panel rects, so the preview can cross-fade across layout segments
  // itself (layoutAt) instead of only ever showing the single static layout above.
  useEffect(() => {
    let live = true;
    previewLayouts(folder).then((d) => { if (live) setLayoutPresets(d); }).catch(() => {});
    return () => { live = false; };
  }, [folder, rev]);
  // Click ripples never change with edits, so fetch once per folder (refetching on every edit
  // was part of the add-effect lag).
  useEffect(() => {
    let live = true;
    clickTrack(folder).then((d) => { if (live) setClicks(d); }).catch(() => {});
    return () => { live = false; };
  }, [folder]);
  // The export background: refetch only when the doc's OWN background settings change (same
  // "don't refetch on every unrelated edit" reasoning as cursorSprites below) - a background
  // panel edit is the only kind of change that can actually alter what `preview_bg` returns.
  useEffect(() => {
    let live = true;
    previewBg(folder).then((u) => { if (live) setBgUrl(u); }).catch(() => {});
    return () => { live = false; };
  }, [folder, JSON.stringify(doc?.settings.background)]);
  // Cursor type track and the record-time "is the OS cursor baked in?" flag: both are properties
  // of the RECORDING, not of the doc, so they never change with edits - fetch once per folder.
  // `osCursor` defaults to true (draw nothing), the safe answer while the fetch is in flight.
  useEffect(() => {
    let live = true;
    cursorKinds(folder).then((d) => { if (live) setCursorKnd(d); }).catch(() => {});
    osCursorInVideo(folder).then((v) => { if (live) setOsCursor(v); }).catch(() => {});
    return () => { live = false; };
  }, [folder]);
  // Cursor sprite pack: refetch when the doc's selected pack changes (picking a different pack,
  // or importing one, in CursorPanel) - NOT on generic `rev` bumps, so unrelated edits don't
  // re-decode sprites (same "don't refetch on every edit" reasoning as clickTrack/previewBg above).
  useEffect(() => {
    if (!doc) return;
    let live = true;
    cursorSprites(folder).then((s) => { if (live) setCursorSpr(s); }).catch(() => {});
    return () => { live = false; };
  }, [folder, doc?.settings.cursor.pack]);
  // Timeline media: filmstrip thumbnails, the system/mic waveform images, and the mixed audio.
  // `wavesReady` distinguishes "still fetching" (AudioTrack shows a shimmer) from "resolved, and
  // this project genuinely has no system/mic audio" (AudioTrack renders nothing) - `waves.system`/
  // `waves.mic` are both `""` in EITHER case, so that state alone can't tell them apart. Thumbs
  // don't need the same treatment: a real recording always has at least one frame, so `!thumbs.
  // length` unambiguously means "still loading" on its own.
  const [wavesReady, setWavesReady] = useState(false);
  useEffect(() => {
    let live = true;
    setWavesReady(false);
    ensureThumbs(folder, 16).then((p) => { if (live) setThumbs(p.map(fileSrc)); }).catch(() => {});
    Promise.all([ensureWaveform(folder, "system"), ensureWaveform(folder, "mic")])
      .then(([s, m]) => { if (live) setWaves({ system: s ? fileSrc(s) : "", mic: m ? fileSrc(m) : "" }); })
      .catch(() => {})
      .finally(() => { if (live) setWavesReady(true); });
    ensurePreviewAudio(folder).then((p) => { if (live) setAudioUrl(p ? fileSrc(p) : ""); }).catch(() => {});
    return () => { live = false; };
  }, [folder]);
  // Transcode (once, cached) a low-res preview proxy and play that instead of the raw 4K.
  // Fast load: show the raw capture instantly (no transcode wait), then hot-swap to the light
  // proxy once it has transcoded (for smoother scrubbing). Pause first so the src remount can't
  // restart playback from 0. The branching itself (raw fast-path vs known-proxy vs fetch) is
  // `planProxySrc` (editorData.ts), a pure helper so it's unit-testable without mocking IPC.
  const proxyReadyRef = useRef(false);
  const lastFolderRef = useRef(folder);
  // Bumped by `retryMedia` (Stage's error-card Retry button) and included in the proxy effect's
  // own deps below, so a retry re-runs it even though `folder`/`quality`/`manifest` haven't
  // changed - on the `fetch` branch this re-issues `ensureProxy`, a genuinely fresh attempt (the
  // relevant case: a legacy/non-preprocessed project, or a non-default quality pick). On the
  // `immediate`/`known` branches the resolved filename is deterministic and unchanged by a retry,
  // so `<video>`'s actual reload there is Stage's own `screen.current?.load()` call.
  const [reloadTick, setReloadTick] = useState(0);
  const retryMedia = () => setReloadTick((t) => t + 1);
  useEffect(() => {
    setPlaying(false);
    if (lastFolderRef.current !== folder) { lastFolderRef.current = folder; proxyReadyRef.current = false; }
    if (!manifest.ready) return; // wait until we know `preprocessed`, so a preprocessed project never loads raw 4K first
    const plan = planProxySrc(quality, DEFAULT_PROXY_HEIGHT, manifest.preprocessed, proxyReadyRef.current);
    if (plan.immediate) setSrcUrl(fileSrc(`${folder}\\${plan.immediate}`));
    if (plan.known) {
      setSrcUrl(fileSrc(`${folder}\\${plan.known}`));
      proxyReadyRef.current = true;
      return;
    }
    if (!plan.fetch) return;
    let live = true;
    ensureProxy(folder, quality).then((p) => { if (live) { setSrcUrl(fileSrc(p)); proxyReadyRef.current = true; } }).catch(() => {});
    return () => { live = false; };
  }, [folder, quality, manifest, reloadTick]);

  useEffect(() => {
    const subs = [
      listen<number>("export-progress", (e) => setPct(e.payload)),
      // Payload is the exported file's own absolute path (`<folder>/final.<ext>`, see run.rs),
      // not just the project folder - stored so ExportProgress can offer "Show in folder".
      listen<string>("export-done", (e) => { setExporting(false); setExportDone(true); setExportPath(e.payload); }),
      listen<string>("export-error", (e) => { setExporting(false); setExportError(e.payload); }),
    ];
    return () => { subs.forEach((s) => s.then((f) => f())); };
  }, []);

  return {
    doc, setDoc, track, layout, layoutPresets, clicks, bgUrl, cursorSpr, cursorKnd, osCursor,
    thumbs, waves, wavesReady, audioUrl, srcUrl, playing, setPlaying, exporting, setExporting, pct, setPct,
    exportDone, setExportDone, exportError, setExportError, exportPath, setExportPath, retryMedia,
  };
}
