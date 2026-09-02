import { memo, useEffect, useRef, useState, type RefObject } from "react";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import type { CursorSettings, ClickFxSettings, ZoomSettings } from "../../hud/settings/settings";
import type { Aspect, CameraMove, EffectRegion, LayoutSeg, Zoom } from "../../lib/edit";
import type { Tab } from "../shell/Rail";
import { camAt } from "./camera";
import { type CamPose } from "./cameraMoves";
import { newSpotlightSimState, spotlightEffectsKey, type SpotlightSimState } from "./spotlightPreview";
import { isNaturalPlaybackTick } from "./playbackTick";
import { useReticleDrag } from "./useReticleDrag";
import { useCompositeLoop } from "../hooks/useCompositeLoop";
import { useCursorSprites } from "../hooks/useCursorSprites";
import { useMediaPlayback } from "../hooks/useMediaPlayback";
import { useSyncRefs } from "../hooks/useSyncRefs";
import { mapCanvasClickToZoomTarget, mapZoomTargetToCanvasPoint } from "./zoomTargetMapper";
import { CamDragHandle } from "./CamDragHandle";
import { ZoomReticle } from "./ZoomReticle";
import { StageToolbar } from "./StageToolbar";
import { StageMedia } from "./StageMedia";
import { Spin } from "../controls/Spin";

const MEDIA_ERR = ["", "aborted", "network", "decode", "src not supported (asset protocol blocked?)"];
// Fallback backing-store size before `layout.canvas` loads (matches the old hardcoded default,
// so the very first paint is unchanged); once loaded, `layout.canvas` (from `PreviewLayout`,
// resolved server-side via `Layout::resolve` from `EditDoc.aspect`) drives the real size.
const DEFAULT_CANVAS: [number, number] = [1280, 720];

/** Smooth, full-composite preview: the screen (a low-res proxy) and webcam play in hidden
 *  native <video>s, and each animation frame is composited onto a 2D canvas (background +
 *  rounded zoomed screen + webcam PiP) via drawPreview. The zoom comes from the exact
 *  camera_track curve. Native decode + Canvas2D drawImage = 60fps. `React.memo`'d (render
 *  hygiene pass) - still re-renders every tick while playing (`timeMs` genuinely drives the
 *  reticle/dirty-tracking), but skips re-rendering for unrelated `Editor` state as long as the
 *  caller passes stable callback props. */
export const Stage = memo(function Stage({ src, webcamSrc, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, bgUrl, cursorSprites, cursorKinds, osCursorInVideo, cursor, effects, clickfx, audioSrc, muted, volume, timeMs, playing, moveMode, aimPoint, aimMode, camDraftRef, tab, onTab, aspect, onAspect, aspectLocked, onTime, onDuration, onZoomAt, onAimAt, onRetryMedia }: {
  src: string; webcamSrc: string; track: CamSample[]; layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[]; cameraMoves: CameraMove[];
  zooms: Zoom[]; zoomSettings: ZoomSettings;
  clicks: ClickSample[]; bgUrl: string;
  cursorSprites: CursorSpriteDto[]; cursorKinds: CursorKindSample[]; osCursorInVideo: boolean; cursor: CursorSettings;
  effects: EffectRegion[]; clickfx: ClickFxSettings;
  audioSrc: string; muted: boolean; volume: number; timeMs: number;
  playing: boolean; moveMode: boolean;
  // StageToolbar's real targets (Task 11 - see StageToolbar.tsx's own doc comment): the active
  // Rail tab (Cursor/Captions/Camera quick-open) and the doc's aspect ratio (quick-cycle, mirrors
  // Transport's chip). Threaded straight through rather than lifting StageToolbar out of Stage -
  // Toast already set the "sibling, not owned by Stage" precedent for NEW state; this is instead
  // just wiring an already-owned child up to state Editor.tsx already lifts.
  tab: Tab; onTab: (t: Tab) => void; aspect: Aspect; onAspect: (a: Aspect) => void; aspectLocked: boolean;
  /** The selected zoom's stored Region aim point (0..1 screen-content), or null when it follows
   *  the cursor / nothing is selected - drives the on-stage reticle. */
  aimPoint: [number, number] | null;
  /** Aim mode: the canvas re-aims the selected zoom instead of adding a new one. */
  aimMode: boolean;
  camDraftRef: RefObject<CamPose | null>;
  onTime: (ms: number) => void; onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void; onAimAt: (x: number, y: number) => void;
  onRetryMedia: () => void;
}) {
  const screen = useRef<HTMLVideoElement>(null);
  const webcam = useRef<HTMLVideoElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [err, setErr] = useState<string | null>(null);
  const bgImg = useRef<HTMLImageElement | null>(null);
  const dirtyRef = useRef(true); // paused: recomposite once per change, not 60fps over a static frame
  // The backing-store size (canvas width/height + .e-stage's aspect-ratio) follows the resolved
  // aspect from the backend (`layout.canvas`) - every fraction in this file (camera rects, click
  // positions, drag mapping) is relative to this same basis, so it must stay a single source.
  const [canvasW, canvasH] = layout?.canvas ?? DEFAULT_CANVAS;

  // Decode the export background (a data URL) once per change into an <img> the canvas draws.
  useEffect(() => {
    if (!bgUrl) { bgImg.current = null; dirtyRef.current = true; return; }
    const img = new Image();
    img.onload = () => { dirtyRef.current = true; };
    img.src = bgUrl;
    bgImg.current = img;
  }, [bgUrl]);

  // Plain-OS fallback, mirroring Rust cursorset::draw: System on a video with no baked cursor
  // draws the synthetic one plainly - arrow only (empty kind track), no bounce, no trail. The raw
  // path itself comes from camera_track, which Rust already leaves unsmoothed in this mode.
  const plainOs = cursor.style === "system" && !osCursorInVideo;
  const effCursor: CursorSettings = plainOs ? { ...cursor, style: "enhanced", click_bounce: false, motion_blur: 0 } : cursor;
  const {
    playRef, timeRef, onTimeRef, trackRef, layoutRef,
    clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  } = useSyncRefs({
    playing, timeMs, onTime, track, layout, clicks, effects, clickfx, cursorKinds: plainOs ? [] : cursorKinds, cursor: effCursor
  });
  // Mirrored into refs directly (not via useSyncRefs) so the rAF loop always reads the live
  // presets/segments without re-subscribing; same plain "useRef + assign each render" pattern.
  const layoutPresetsRef = useRef(layoutPresets); layoutPresetsRef.current = layoutPresets;
  const layoutSegsRef = useRef(layoutSegs); layoutSegsRef.current = layoutSegs;
  const cameraMovesRef = useRef(cameraMoves); cameraMovesRef.current = cameraMoves;
  // Zooms + zoom settings drive the smart webcam-on-zoom action (mirrors step_camera).
  const zoomsRef = useRef(zooms); zoomsRef.current = zooms;
  const zoomSettingsRef = useRef(zoomSettings); zoomSettingsRef.current = zoomSettings;
  // camDraftRef (lifted to Editor, shared with CameraPanel's save button) holds the UNSAVED Move-
  // mode pose: the loop draws the PiP here when non-null. CamDragHandle updates it live but does
  // NOT commit a keyframe - only the panel's Update/Add button saves it, and moving the playhead
  // discards it (the effect below).
  const trailRef = useRef<[number, number][]>([]);
  const onSpriteLoaded = () => { dirtyRef.current = true; };
  const spritesRef = useCursorSprites(cursorSprites, onSpriteLoaded);
  // Created here (not inside useCompositeLoop) so it can ALSO be reset below, on a paused effects
  // edit the loop's own discontinuous-jump gate can't see (that gate only fires on a moving `t`).
  const spotSimRef = useRef<SpotlightSimState>(newSpotlightSimState());

  // Mark the canvas dirty on any draw-affecting change so the PAUSED rAF recomposites exactly once
  // per change instead of redrawing the same static frame at 60fps (the idle/interaction-lag fix).
  useEffect(() => { dirtyRef.current = true; }, [timeMs, playing, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, effects, cursor, clickfx, cursorKinds]);
  // Gate 2 for the "spotlight freezes mid-fade" bug: a retime/add/remove of a Spotlight region
  // WHILE PAUSED doesn't move `timeMs` at all, so useCompositeLoop's discontinuous-jump reset
  // (gate 1) never fires - the sim's in-flight fade-out (armed when the old driver disappeared)
  // would otherwise keep easing forever against a frozen paused `t`. Keyed on CONTENT
  // (`spotlightEffectsKey`), not the `effects` array reference, mirroring CamDragHandle's
  // `cameraMovesKey` use just below for the identical reason (`applyEditOp` hands back a new
  // array reference on every edit routed through it, not just spotlight ones).
  const spotEffectsKeyRef = useRef(spotlightEffectsKey(effects));
  useEffect(() => {
    const key = spotlightEffectsKey(effects);
    if (key !== spotEffectsKeyRef.current) spotSimRef.current = newSpotlightSimState();
    spotEffectsKeyRef.current = key;
  }, [effects]);
  // Moving the playhead discards the unsaved Move-mode draft - UNLESS this `timeMs` change is
  // just the composite loop's own natural playback progress (`isNaturalPlaybackTick`), not an
  // actual seek/scrub (M6, review round 1 Important 3). The draft must survive an ordinary tick
  // whether the drag is still in progress OR was already released - "nothing is saved until you
  // press the button" (CameraPanel's own promise) only holds if the draft actually lives until
  // then, not just until the next ~16/sec tick after letting go. `playRef` (from `useSyncRefs`
  // above) keeps this reading the CURRENT `playing`, not whatever it was when the effect last ran.
  const lastCamTimeRef = useRef(timeMs);
  useEffect(() => {
    dirtyRef.current = true;
    if (!isNaturalPlaybackTick(lastCamTimeRef.current, timeMs, playRef.current)) camDraftRef.current = null;
    lastCamTimeRef.current = timeMs;
  }, [timeMs]); // eslint-disable-line react-hooks/exhaustive-deps

  useMediaPlayback({ screenRef: screen, webcamRef: webcam, audioRef: audio, playing, src, muted, volume, audioSrc, timeMs, playRef });

  useCompositeLoop({
    screenRef: screen, webcamRef: webcam, audioRef: audio, canvasRef: canvas,
    playRef, timeRef, onTimeRef,
    trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, cameraMovesRef, zoomsRef, zoomSettingsRef, dragPoseRef: camDraftRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
    spritesRef, trailRef, dirtyRef, bgImgRef: bgImg, spotSimRef,
  });

  // Inverse-map a pointer position through the current zoom crop + screen rect to a 0..1
  // screen-content fraction (the zoom target's own basis).
  const targetUnderPointer = (clientX: number, clientY: number) => {
    const c = canvas.current, sv = screen.current; if (!c || !sv) return null;
    if (sv.videoWidth <= 0 || sv.videoHeight <= 0) return null;
    return mapCanvasClickToZoomTarget({ clientX, clientY, canvasElement: c, layout: layoutRef.current,
      cam: camAt(trackRef.current, timeRef.current) });
  };
  // Clicking the preview adds a zoom focused on that point - EXCEPT in aim mode, where it re-aims
  // the already-selected Region zoom instead (the two are mutually exclusive click meanings).
  const onCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const t = targetUnderPointer(e.clientX, e.clientY);
    if (t) (aimMode ? onAimAt : onZoomAt)(t[0], t[1]);
  };
  // Dragging the reticle re-aims continuously, debounced (`useReticleDrag`, a sibling hook -
  // extracted purely to keep this file under its line budget; see its own doc comment and
  // `Stage.md`'s "Reticle drag debounce" for the full write-up).
  const { liveAim, aimDrag, onReticleDown } = useReticleDrag(aimPoint, onAimAt, targetUnderPointer);
  // Where that stored aim point lands on the canvas RIGHT NOW (the crop moves it as the camera
  // ramps). Hidden during playback: the reticle is an editing affordance, not a playback overlay.
  const shownAim = liveAim ?? aimPoint;
  const reticle = shownAim && !playing
    ? mapZoomTargetToCanvasPoint({ tx: shownAim[0], ty: shownAim[1], canvasW, canvasH, layout, cam: camAt(track, timeMs) })
    : null;

  return (
    <div className="e-stagewrap">
      <StageToolbar tab={tab} onTab={onTab} aspect={aspect} onAspect={onAspect} aspectLocked={aspectLocked} />
      <div className="e-stage" style={{ aspectRatio: `${canvasW} / ${canvasH}` }}>
        {!src && !err && <div className="e-stage-empty"><Spin size={20} /><span>Preparing preview</span></div>}
        <canvas ref={canvas} className="e-canvas" width={canvasW} height={canvasH} onClick={onCanvasClick}
          title={aimMode ? "Click to aim this zoom" : "Click to add a zoom here"}
          style={{ display: src ? "block" : "none", cursor: aimMode ? "crosshair" : "zoom-in" }} />
        {reticle && <ZoomReticle x={reticle[0]} y={reticle[1]} aiming={aimMode} dragging={aimDrag}
          onPointerDown={aimMode ? onReticleDown : undefined} />}
        {moveMode && (
          <CamDragHandle layout={layout} layoutPresets={layoutPresets} layoutSegs={layoutSegs} cameraMoves={cameraMoves}
            timeMs={timeMs} playing={playing} canvasW={canvasW} canvasH={canvasH} canvasRef={canvas} camDraftRef={camDraftRef} dirtyRef={dirtyRef} />
        )}
        <StageMedia screenRef={screen} webcamRef={webcam} audioRef={audio} src={src} webcamSrc={webcamSrc} audioSrc={audioSrc} err={err}
          onScreenLoadedData={() => { setErr(null); dirtyRef.current = true; }} onScreenSeeked={() => { dirtyRef.current = true; }}
          onScreenEnded={() => { const v = screen.current; if (v && isFinite(v.duration)) onTimeRef.current(Math.round(v.duration * 1000)); }}
          onScreenLoadedMetadata={(e) => { const v = e.currentTarget; const d = v.duration; if (isFinite(d) && d > 0) onDuration(Math.round(d * 1000));
            v.currentTime = Math.max(0, timeRef.current / 1000); /* restore position across the raw->proxy swap */ if (playRef.current) v.play().catch(() => {}); }}
          onScreenError={(e) => setErr(MEDIA_ERR[e.currentTarget.error?.code ?? 0] || "load failed")}
          onWebcamLoadedData={() => { dirtyRef.current = true; }} onWebcamSeeked={() => { dirtyRef.current = true; }}
          onRetry={() => { setErr(null); onRetryMedia(); screen.current?.load(); }} />
      </div>
    </div>
  );
});
