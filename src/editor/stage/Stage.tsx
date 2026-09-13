import { memo, useRef, useState, type RefObject } from "react";
import type { CamSample, ClickSample, CursorPackDto, CursorKindSample, CursorLayerDto, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import type { CursorSettings, ClickFxSettings, ZoomSettings } from "../../hud/settings/settings";
import type { Aspect, CameraMove, EditDoc, EditOp, EffectRegion, LayoutSeg, Zoom } from "../../lib/edit";
import type { Tab } from "../shell/panelTabs";
import { camAt } from "./camera";
import { type CamPose } from "./cameraMoves";
import { newSpotlightSimState, type SpotlightSimState } from "./spotlightPreview";
import { useReticleDrag } from "./useReticleDrag";
import { useStageInvalidation } from "./useStageInvalidation";
import type { StageBg, StageBgState } from "./stageBg";
import { useArrangeDrag } from "./arrange/useArrangeDrag";
import { ArrangeOverlay } from "./arrange/ArrangeOverlay";
import { useCompositeLoop } from "../hooks/useCompositeLoop";
import { useCursorSprites } from "../hooks/useCursorSprites";
import { useMediaPlayback } from "../hooks/useMediaPlayback";
import { useSyncRefs } from "../hooks/useSyncRefs";
import { outOf, type TimeMap } from "../../lib/remap";
import { stageCursor } from "./stageCursor";
import { mapCanvasClickToZoomTarget, mapZoomTargetToCanvasPoint } from "./zoomTargetMapper";
import { CamDragHandle } from "./CamDragHandle";
import { ZoomReticle } from "./ZoomReticle";
import { StageToolbar } from "./StageToolbar";
import { StageMedia } from "./StageMedia";
import { StageEmpty } from "./StageEmpty";

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
export const Stage = memo(function Stage({ src, webcamSrc, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, bg, map, cursorSprites, cursorKinds, cursorLayer, osCursorInVideo, cursor, effects, clickfx, audioSrc, muted, volume, timeMs, playing, moveMode, aimPoint, aimMode, arrangeSeg, camDraftRef, tab, onTab, aspect, onAspect, aspectLocked, onTime, onDuration, onZoomAt, onAimAt, onApply, onRetryMedia }: {
  src: string; webcamSrc: string; track: CamSample[]; layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[]; cameraMoves: CameraMove[];
  zooms: Zoom[]; zoomSettings: ZoomSettings;
  clicks: ClickSample[]; bg: StageBg; map: TimeMap;
  cursorSprites: CursorPackDto | null; cursorKinds: CursorKindSample[]; cursorLayer: CursorLayerDto | null; osCursorInVideo: boolean; cursor: CursorSettings;
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
  /** The layout segment being arranged on the stage, else null. Non-null IS arrange mode - a third
   *  exclusive stage mode alongside aim and Move (all three claim the same pointer), so it
   *  suspends click-to-zoom/aim and hides the Move handle for as long as it is on. */
  arrangeSeg: LayoutSeg | null;
  camDraftRef: RefObject<CamPose | null>;
  onTime: (ms: number) => void; onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void; onAimAt: (x: number, y: number) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onRetryMedia: () => void;
}) {
  const screen = useRef<HTMLVideoElement>(null);
  const webcam = useRef<HTMLVideoElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [err, setErr] = useState<string | null>(null);
  const bgImg = useRef<StageBgState | null>(null); // the still PNG + any moving asset, owned by useStageInvalidation
  const dirtyRef = useRef(true); // paused: recomposite once per change, not 60fps over a static frame
  // The backing-store size (canvas width/height + .e-stage's aspect-ratio) follows the resolved
  // aspect from the backend (`layout.canvas`) - every fraction in this file (camera rects, click
  // positions, drag mapping) is relative to this same basis, so it must stay a single source.
  const [canvasW, canvasH] = layout?.canvas ?? DEFAULT_CANVAS;

  // "System" on a recording that captured the real cursor as its own layer composites THAT
  // (mirroring Rust `captured::draws_captured`); the style stays "system" so `drawCursorSprite`
  // takes the captured branch. Only a PRE-LAYER recording with no baked cursor falls back to the
  // plain-OS synthetic arrow (empty kind track, no bounce, no trail); the raw path itself comes
  // from camera_track, which Rust already leaves unsmoothed in that mode.
  const { captured, plainOs, effCursor } = stageCursor(cursor, osCursorInVideo, cursorLayer);
  const tOut = outOf(map, timeMs); const mapRef = useRef(map); mapRef.current = map; // the track and every region sit on the output clock; the playhead is clip time
  const {
    playRef, timeRef, onTimeRef, trackRef, layoutRef,
    clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  } = useSyncRefs({
    playing, timeMs, onTime, track, layout, clicks, effects, clickfx, cursorKinds: plainOs ? [] : cursorKinds, cursor: effCursor
  });
  // Arrange mode's live pose draft. It is folded back into `LayoutPresets.segs` (`arrangePresets`)
  // rather than given a drawing path of its own, so the loop below - and `layoutAt`'s cross-fades -
  // see a dragged segment exactly as they will once it commits. Presentation only: the op is
  // written on release (see useArrangeDrag).
  const arrange = useArrangeDrag({ seg: arrangeSeg, presets: layoutPresets, canvasRef: canvas, canvasW, canvasH, dirtyRef, onApply });
  const arranging = arrangeSeg !== null;
  // Mode exclusivity has a half beyond the pointer: `frameCamLayout` gives `camDraftRef` precedence
  // over the base layout rect (`drag ?? camMoveAt(...)`), which is exactly where the arrange draft
  // lives - a leftover UNSAVED Move drag would pin the composited webcam while the arrange frame
  // moved freely. The loop therefore reads the draft through `activeCamDraft`, which SUPPRESSES it
  // while this is true instead of clearing it: `camDraftRef` is never written here, so the Move
  // draft survives arrange mode and reasserts on exit, still discarded only by its own two
  // documented triggers (CameraPanel's Add/Update, or moving the playhead).
  const arrangingRef = useRef(arranging); arrangingRef.current = arranging;
  // Mirrored into refs directly (not via useSyncRefs) so the rAF loop always reads the live
  // presets/segments without re-subscribing; same plain "useRef + assign each render" pattern.
  const layoutPresetsRef = useRef(arrange.presets); layoutPresetsRef.current = arrange.presets;
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
  const spritesRef = useCursorSprites(cursorSprites, captured, onSpriteLoaded);
  // Created here (not inside useCompositeLoop) so it can ALSO be reset below, on a paused effects
  // edit the loop's own discontinuous-jump gate can't see (that gate only fires on a moving `t`).
  const spotSimRef = useRef<SpotlightSimState>(newSpotlightSimState());

  // The background decode + all three "recomposite / drop the draft now" gates (see the hook's
  // own doc comment) - extracted to keep this file under its line budget, like useReticleDrag.
  useStageInvalidation({ bg, bgRef: bgImg, dirtyRef, effects, spotSimRef, timeMs: tOut, playRef, camDraftRef,
    drawDeps: [timeMs, playing, track, layout, arrange.presets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, effects, cursor, clickfx, cursorKinds, captured, arranging] });

  useMediaPlayback({ screenRef: screen, webcamRef: webcam, audioRef: audio, playing, src, muted, volume, audioSrc, timeMs, playRef });

  useCompositeLoop({
    screenRef: screen, webcamRef: webcam, audioRef: audio, canvasRef: canvas,
    playRef, timeRef, onTimeRef,
    trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, cameraMovesRef, zoomsRef, zoomSettingsRef, dragPoseRef: camDraftRef, arrangingRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
    spritesRef, trailRef, dirtyRef, bgRef: bgImg, spotSimRef, mapRef,
  });

  // Inverse-map a pointer position through the current zoom crop + screen rect to a 0..1
  // screen-content fraction (the zoom target's own basis).
  const targetUnderPointer = (clientX: number, clientY: number) => {
    const c = canvas.current, sv = screen.current; if (!c || !sv) return null;
    if (sv.videoWidth <= 0 || sv.videoHeight <= 0) return null;
    return mapCanvasClickToZoomTarget({ clientX, clientY, canvasElement: c, layout: layoutRef.current,
      cam: camAt(trackRef.current, outOf(mapRef.current, timeRef.current)) });
  };
  // Clicking the preview adds a zoom focused on that point - EXCEPT in aim mode, where it re-aims
  // the already-selected Region zoom instead (the two are mutually exclusive click meanings), and
  // except while arranging, where the click belongs to the panel frames and must not also drop a
  // zoom behind them (the same guard aim mode gets, one mode further).
  const onCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (arranging) return;
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
  const reticle = shownAim && !playing && !arranging
    ? mapZoomTargetToCanvasPoint({ tx: shownAim[0], ty: shownAim[1], canvasW, canvasH, layout, cam: camAt(track, tOut) })
    : null;

  return (
    <div className="e-stagewrap">
      <StageToolbar tab={tab} onTab={onTab} aspect={aspect} onAspect={onAspect} aspectLocked={aspectLocked} />
      <div className="e-stage" style={{ aspectRatio: `${canvasW} / ${canvasH}` }}>
        {!src && !err && <StageEmpty />}
        <canvas ref={canvas} className="e-canvas" width={canvasW} height={canvasH} onClick={onCanvasClick}
          title={arranging ? "Drag the panel frames to arrange this layout" : aimMode ? "Click to aim this zoom" : "Click to add a zoom here"}
          style={{ display: src ? "block" : "none", cursor: arranging ? "default" : aimMode ? "crosshair" : "zoom-in" }} />
        {reticle && <ZoomReticle x={reticle[0]} y={reticle[1]} aiming={aimMode} dragging={aimDrag}
          onPointerDown={aimMode ? onReticleDown : undefined} />}
        {moveMode && !arranging && (
          <CamDragHandle layout={layout} layoutPresets={arrange.presets} layoutSegs={layoutSegs} cameraMoves={cameraMoves}
            timeMs={tOut} playing={playing} canvasW={canvasW} canvasH={canvasH} canvasRef={canvas} camDraftRef={camDraftRef} dirtyRef={dirtyRef} />
        )}
        {arrangeSeg && arrange.panels && (
          <ArrangeOverlay seg={arrangeSeg} panels={arrange.panels} camMoves={cameraMoves} guideX={arrange.guideX}
            guideY={arrange.guideY} active={arrange.active} onPanelDown={arrange.onPanelDown} onHideCam={arrange.onHideCam} />
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
