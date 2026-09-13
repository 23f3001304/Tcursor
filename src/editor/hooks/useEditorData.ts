import { useCallback, useEffect, useRef, useState } from "react";
import { getEdit, setCapturable, cameraTrack, previewLayout, previewLayouts, clickTrack, previewBg, cursorSprites, cursorKinds, cursorLayer, osCursorInVideo, ensureThumbs, ensureWaveform, ensurePreviewAudio, ensureProxy, fileSrc, getProjectManifest, DEFAULT_PROXY_HEIGHT } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";
import type { CamSample, ClickSample, CursorPackDto, CursorKindSample, CursorLayerDto, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import { planProxySrc } from "./editorData";
import { debounce } from "./debounce";
import { bgAssetUrl, type StageBg } from "../stage/stageBg";

// Trailing debounce window for the previewBg refetch below - see editor.md "render hygiene".
const PREVIEW_BG_DEBOUNCE_MS = 80;

// All the editor's preview/timeline data fetching, split out of Editor so the component
// itself only holds render + mutation logic. See docs/api/src/editor/Editor.md for why each
// fetch is keyed on [folder], [folder, rev], or [folder, quality]. Export-run state lives in the
// sibling useExportState.ts instead (kept separate so this file stays under its line budget).
export function useEditorData(folder: string, rev: number, quality: number) {
  const [doc, setDoc] = useState<EditDoc | null>(null);
  const [track, setTrack] = useState<CamSample[]>([]);
  const [layout, setLayout] = useState<PreviewLayout | null>(null);
  const [layoutPresets, setLayoutPresets] = useState<LayoutPresets | null>(null);
  const [clicks, setClicks] = useState<ClickSample[]>([]);
  const [bgUrl, setBgUrl] = useState("");
  const [cursorSpr, setCursorSpr] = useState<CursorPackDto | null>(null);
  const [cursorKnd, setCursorKnd] = useState<CursorKindSample[]>([]);
  const [cursorLyr, setCursorLyr] = useState<CursorLayerDto | null>(null);
  const [osCursor, setOsCursor] = useState(true);
  const [thumbs, setThumbs] = useState<string[]>([]);
  const [waves, setWaves] = useState<{ system: string; mic: string }>({ system: "", mic: "" });
  const [audioUrl, setAudioUrl] = useState("");
  const [srcUrl, setSrcUrl] = useState("");
  const [playing, setPlaying] = useState(false);

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
  // The export background: refetch only when the doc's OWN background settings change - a
  // background panel edit is the only kind of change that can actually alter what `preview_bg`
  // returns. Debounced (trailing): a background slider drag changes this dependency at up to the
  // Slider's own commit rate, and `preview_bg` re-encodes/base64s the full preview background -
  // not something to redo dozens of times a second (see editor.md "render hygiene"). `fetchBgRef`
  // is a lazy-initialized singleton (NOT `useRef(debounce(...))` - that form still calls
  // `debounce(...)` on every render just to discard the result) so a burst of doc changes
  // collapses into one fetch; `bgSeqRef`/`bgLiveRef` reproduce the discarded-stale-response
  // guarantee the other fetches here get from their own per-effect `live` flag, since this fetch
  // now outlives any single effect run.
  const bgSeqRef = useRef(0);
  const bgLiveRef = useRef(true);
  const fetchBgRef = useRef<ReturnType<typeof debounce<[string]>> | null>(null);
  if (!fetchBgRef.current) {
    fetchBgRef.current = debounce((f: string) => {
      const seq = ++bgSeqRef.current;
      previewBg(f).then((u) => { if (bgLiveRef.current && bgSeqRef.current === seq) setBgUrl(u); }).catch(() => {});
    }, PREVIEW_BG_DEBOUNCE_MS);
  }
  useEffect(() => { fetchBgRef.current!(folder); }, [folder, JSON.stringify(doc?.settings.background)]);
  // Resets `bgLiveRef` true on EVERY mount, not just at declaration - React 19 StrictMode
  // (`main.tsx`) double-invokes effects in dev (mount -> cleanup -> mount, same fiber), so
  // `useRef(true)`'s initial value only applies to the FIRST pass; without this reset, the dev-
  // only cleanup pass permanently flips it false and every response is discarded thereafter -
  // `bgUrl` never leaves `""` in dev.
  useEffect(() => { bgLiveRef.current = true; return () => { bgLiveRef.current = false; fetchBgRef.current?.cancel(); }; }, []);
  // Cursor type track and the record-time "is the OS cursor baked in?" flag: both are properties
  // of the RECORDING, not of the doc, so they never change with edits - fetch once per folder.
  // `osCursor` defaults to true (draw nothing), the safe answer while the fetch is in flight.
  // The captured OS-cursor layer rides along: same "property of the recording" reasoning, and
  // `null` (no layer - a pre-layer recording) is the safe answer while the fetch is in flight.
  useEffect(() => {
    let live = true;
    cursorKinds(folder).then((d) => { if (live) setCursorKnd(d); }).catch(() => {});
    osCursorInVideo(folder).then((v) => { if (live) setOsCursor(v); }).catch(() => {});
    cursorLayer(folder).then((l) => { if (live) setCursorLyr(l); }).catch(() => {});
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
  // `useCallback`'d (render hygiene pass, deps `[]` - `setReloadTick` is a `useState` setter, so
  // it's already stable forever) so `retryMedia`'s identity holds across renders, which `Stage`
  // (`React.memo`'d) needs to actually skip re-rendering for it.
  const retryMedia = useCallback(() => setReloadTick((t) => t + 1), []);
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

  // Everything the stage needs to paint the background, as ONE value: the backend's finished PNG
  // for the static case, plus the imported asset's URL for the moving one. Bundled here (rather
  // than threaded as four props) so `Stage` and `useCompositeLoop` kept their existing signatures
  // when video backgrounds landed - see `stageBg.ts`.
  const bgSet = doc?.settings.background;
  const bg: StageBg = {
    url: bgUrl,
    assetUrl: bgAssetUrl(folder, bgSet?.asset, bgSet?.kind ?? "mesh", fileSrc),
    assetPath: bgSet?.asset ?? "",
    kind: bgSet?.kind ?? "mesh",
    dim: bgSet?.dim ?? 0,
  };

  return {
    doc, setDoc, track, layout, layoutPresets, clicks, bg, cursorSpr, cursorKnd, cursorLyr, osCursor,
    thumbs, waves, wavesReady, audioUrl, srcUrl, playing, setPlaying, retryMedia,
  };
}
