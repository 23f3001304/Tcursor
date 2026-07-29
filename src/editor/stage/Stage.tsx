import { useEffect, useRef, useState, type RefObject } from "react";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import type { CursorSettings, ClickFxSettings, ZoomSettings } from "../../hud/settings/settings";
import type { CameraMove, EffectRegion, LayoutSeg, Zoom } from "../../lib/edit";
import { camAt } from "./camera";
import { camMoveAt, rectFromCenter, type CamPose } from "./cameraMoves";
import { mapPointerToCamFraction } from "./camDragMapper";
import { layoutAt } from "../timeline/layoutTrack";
import { useCompositeLoop } from "../hooks/useCompositeLoop";
import { useCursorSprites } from "../hooks/useCursorSprites";
import { useMediaPlayback } from "../hooks/useMediaPlayback";
import { useSyncRefs } from "../hooks/useSyncRefs";
import { mapCanvasClickToZoomTarget } from "./zoomTargetMapper";
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
 *  camera_track curve. Native decode + Canvas2D drawImage = 60fps. */
export function Stage({ src, webcamSrc, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, bgUrl, cursorSprites, cursorKinds, cursor, effects, clickfx, audioSrc, muted, volume, timeMs, playing, moveMode, camDraftRef, onTime, onDuration, onZoomAt }: {
  src: string; webcamSrc: string; track: CamSample[]; layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[]; cameraMoves: CameraMove[];
  zooms: Zoom[]; zoomSettings: ZoomSettings;
  clicks: ClickSample[]; bgUrl: string;
  cursorSprites: CursorSpriteDto[]; cursorKinds: CursorKindSample[]; cursor: CursorSettings;
  effects: EffectRegion[]; clickfx: ClickFxSettings;
  audioSrc: string; muted: boolean; volume: number; timeMs: number;
  playing: boolean; moveMode: boolean;
  camDraftRef: RefObject<CamPose | null>;
  onTime: (ms: number) => void; onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void;
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

  const {
    playRef, timeRef, onTimeRef, trackRef, layoutRef,
    clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  } = useSyncRefs({
    playing, timeMs, onTime, track, layout, clicks, effects, clickfx, cursorKinds, cursor
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
  // mode pose: the loop draws the PiP here when non-null. Dragging updates it live but does NOT
  // commit a keyframe - only the panel's Update/Add button saves it, and moving the playhead
  // discards it (the effect below). dragPose (state) mirrors it to drive the handle's CSS position.
  const [dragPose, setDragPose] = useState<CamPose | null>(null);
  const trailRef = useRef<[number, number][]>([]);
  const onSpriteLoaded = () => { dirtyRef.current = true; };
  const spritesRef = useCursorSprites(cursorSprites, onSpriteLoaded);

  // Mark the canvas dirty on any draw-affecting change so the PAUSED rAF recomposites exactly once
  // per change instead of redrawing the same static frame at 60fps (the idle/interaction-lag fix).
  useEffect(() => { dirtyRef.current = true; }, [timeMs, playing, track, layout, layoutPresets, layoutSegs, cameraMoves, zooms, zoomSettings, clicks, effects, cursor, clickfx, cursorKinds]);
  // Moving the playhead discards the unsaved Move-mode draft - the PiP resets to its sampled pose.
  useEffect(() => { camDraftRef.current = null; setDragPose(null); dirtyRef.current = true; }, [timeMs]); // eslint-disable-line react-hooks/exhaustive-deps

  useMediaPlayback({
    screenRef: screen,
    webcamRef: webcam,
    audioRef: audio,
    playing,
    src,
    muted,
    volume,
    audioSrc,
    timeMs,
    playRef,
  });

  useCompositeLoop({
    screenRef: screen, webcamRef: webcam, audioRef: audio, canvasRef: canvas,
    playRef, timeRef, onTimeRef,
    trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, cameraMovesRef, zoomsRef, zoomSettingsRef, dragPoseRef: camDraftRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
    spritesRef, trailRef, dirtyRef, bgImgRef: bgImg,
  });

  // Click the preview to add a zoom focused on that point: inverse-map the click through the
  // current zoom crop + screen rect to a 0..1 screen-content fraction (the zoom target).
  const onCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const c = canvas.current, sv = screen.current; if (!c || !sv) return;
    if (sv.videoWidth <= 0 || sv.videoHeight <= 0) return;
    const cam = camAt(trackRef.current, timeRef.current);
    const target = mapCanvasClickToZoomTarget({
      clientX: e.clientX,
      clientY: e.clientY,
      canvasElement: c,
      layout: layoutRef.current,
      cam,
    });
    if (target) {
      onZoomAt(target[0], target[1]);
    }
  };

  // The PiP rect (fractions of the canvas) at the current time, for positioning the drag handle -
  // same computation the composite loop makes for `frameLayout.cam` (layoutAt -> camMoveAt ->
  // rectFromCenter), so the handle sits in exact parity with what's actually drawn. `.e-stage`'s
  // box is the canvas's own displayed rect (the stage is sized to the resolved aspect-ratio, a box
  // the canvas fills exactly, no letterbox gap), so these fractions convert straight to CSS
  // percentages of `.e-stage` with no separate client-rect math needed.
  const baseLayout = layoutAt(layoutSegs, layoutPresets, timeMs, [canvasW, canvasH]) ?? layout;
  // The un-overridden static PiP pose (implicit t=0 keyframe a lone camera_moves keyframe eases
  // in from) - same derivation useCompositeLoop.ts makes, so the handle matches the drawn frame.
  const staticPose = baseLayout?.cam
    ? { x: baseLayout.cam[0] + baseLayout.cam[2] / 2, y: baseLayout.cam[1] + baseLayout.cam[3] / 2, size: baseLayout.cam[3] }
    : null;
  const sampledPose = camMoveAt(cameraMoves, timeMs, staticPose);
  const activePose = dragPose ?? sampledPose;
  const pipRect: [number, number, number, number] | null = !baseLayout?.cam ? null
    : activePose ? rectFromCenter(activePose, canvasW, canvasH)
    : [baseLayout.cam[0], baseLayout.cam[1], baseLayout.cam[2], baseLayout.cam[3]];
  // Match the handle's rounding to the webcam shape (radius/width ratio: ~50% circle, frac rounded,
  // 0 rect) so it hugs the PiP instead of a boxy border sticking out past a round webcam.
  const handleRadiusPct = baseLayout?.cam && baseLayout.cam[2] > 0
    ? Math.min(50, (baseLayout.cam[4] / baseLayout.cam[2]) * 100) : 12;

  // Move-mode drag: a pointerdown on the handle records the start but does NOT move or commit
  // anything - the PiP only follows (and a keyframe is only written) once the pointer actually
  // drags past a small threshold. So a plain click on the preview, or scrubbing the playhead,
  // never adds/updates a keyframe - the PiP just shows its sampled pose at the playhead. Window
  // listeners (not React pointer capture) keep the drag alive past the handle; live feedback goes
  // into dragPoseRef (read by the rAF loop each frame), dragPose (state) drives the handle's CSS.
  const onHandlePointerDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    const c = canvas.current; if (!c) return;
    const start = { x: e.clientX, y: e.clientY, size: (camDraftRef.current ?? sampledPose)?.size ?? 0.25 };
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved && Math.hypot(ev.clientX - start.x, ev.clientY - start.y) < 4) return; // ignore a click
      moved = true;
      const [x, y] = mapPointerToCamFraction({ clientX: ev.clientX, clientY: ev.clientY, canvasElement: c });
      // Update the live draft only - NO commit. Saving is the Camera panel's Update/Add button.
      camDraftRef.current = { x, y, size: start.size }; setDragPose({ x, y, size: start.size }); dirtyRef.current = true;
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  return (
    <div className="e-stagewrap">
      <StageToolbar />
      <div className="e-stage" style={{ aspectRatio: `${canvasW} / ${canvasH}` }}>
        {!src && !err && <div className="e-stage-empty"><Spin size={20} /><span>Preparing preview</span></div>}
        <canvas ref={canvas} className="e-canvas" width={canvasW} height={canvasH} onClick={onCanvasClick}
          title="Click to add a zoom here" style={{ display: src ? "block" : "none", cursor: "zoom-in" }} />
        {moveMode && pipRect && (
          <div className={`e-camdrag${dragPose ? " drag" : ""}`}
            style={{ left: `${pipRect[0] * 100}%`, top: `${pipRect[1] * 100}%`, width: `${pipRect[2] * 100}%`, height: `${pipRect[3] * 100}%`, borderRadius: `${handleRadiusPct}%` }}
            title="Drag to reposition the webcam"
            onPointerDown={onHandlePointerDown} />
        )}
        <StageMedia screenRef={screen} webcamRef={webcam} audioRef={audio} src={src} webcamSrc={webcamSrc} audioSrc={audioSrc} err={err}
          onScreenLoadedData={() => { setErr(null); dirtyRef.current = true; }} onScreenSeeked={() => { dirtyRef.current = true; }}
          onScreenEnded={() => { const v = screen.current; if (v && isFinite(v.duration)) onTimeRef.current(Math.round(v.duration * 1000)); }}
          onScreenLoadedMetadata={(e) => { const v = e.currentTarget; const d = v.duration; if (isFinite(d) && d > 0) onDuration(Math.round(d * 1000));
            v.currentTime = Math.max(0, timeRef.current / 1000); /* restore position across the raw->proxy swap */ if (playRef.current) v.play().catch(() => {}); }}
          onScreenError={(e) => setErr(MEDIA_ERR[e.currentTarget.error?.code ?? 0] || "load failed")}
          onWebcamLoadedData={() => { dirtyRef.current = true; }} onWebcamSeeked={() => { dirtyRef.current = true; }} />
      </div>
    </div>
  );
}
