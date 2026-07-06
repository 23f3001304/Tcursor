import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { CamSample, ClickSample, CursorSpriteDto, CursorKindSample, PreviewLayout, LayoutPresets } from "../../lib/ipc";
import type { CursorSettings, ClickFxSettings } from "../../hud/settings/settings";
import type { EffectRegion, LayoutSeg } from "../../lib/edit";
import { camAt } from "./camera";
import { useCompositeLoop } from "../hooks/useCompositeLoop";
import { useCursorSprites } from "../hooks/useCursorSprites";
import { useMediaPlayback } from "../hooks/useMediaPlayback";
import { useSyncRefs } from "../hooks/useSyncRefs";
import { mapCanvasClickToZoomTarget } from "./zoomTargetMapper";
import { StageToolbar } from "./StageToolbar";
import { Spin } from "../controls/Spin";

const MEDIA_ERR = ["", "aborted", "network", "decode", "src not supported (asset protocol blocked?)"];
const HIDDEN: CSSProperties = { position: "absolute", width: 1, height: 1, opacity: 0, pointerEvents: "none" };

/** Smooth, full-composite preview: the screen (a low-res proxy) and webcam play in hidden
 *  native <video>s, and each animation frame is composited onto a 2D canvas (background +
 *  rounded zoomed screen + webcam PiP) via drawPreview. The zoom comes from the exact
 *  camera_track curve. Native decode + Canvas2D drawImage = 60fps. */
export function Stage({ src, webcamSrc, track, layout, layoutPresets, layoutSegs, clicks, bgUrl, cursorSprites, cursorKinds, cursor, effects, clickfx, audioSrc, muted, timeMs, playing, onTime, onDuration, onZoomAt }: {
  src: string; webcamSrc: string; track: CamSample[]; layout: PreviewLayout | null;
  layoutPresets: LayoutPresets | null; layoutSegs: LayoutSeg[];
  clicks: ClickSample[]; bgUrl: string;
  cursorSprites: CursorSpriteDto[]; cursorKinds: CursorKindSample[]; cursor: CursorSettings;
  effects: EffectRegion[]; clickfx: ClickFxSettings;
  audioSrc: string; muted: boolean; timeMs: number;
  playing: boolean; onTime: (ms: number) => void; onDuration: (ms: number) => void;
  onZoomAt: (x: number, y: number) => void;
}) {
  const screen = useRef<HTMLVideoElement>(null);
  const webcam = useRef<HTMLVideoElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [err, setErr] = useState<string | null>(null);
  const bgImg = useRef<HTMLImageElement | null>(null);
  const dirtyRef = useRef(true); // paused: recomposite once per change, not 60fps over a static frame

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
  const trailRef = useRef<[number, number][]>([]);
  const onSpriteLoaded = () => { dirtyRef.current = true; };
  const spritesRef = useCursorSprites(cursorSprites, onSpriteLoaded);

  // Mark the canvas dirty on any draw-affecting change so the PAUSED rAF recomposites exactly once
  // per change instead of redrawing the same static frame at 60fps (the idle/interaction-lag fix).
  useEffect(() => { dirtyRef.current = true; }, [timeMs, playing, track, layout, layoutPresets, layoutSegs, clicks, effects, cursor, clickfx, cursorKinds]);

  useMediaPlayback({
    screenRef: screen,
    webcamRef: webcam,
    audioRef: audio,
    playing,
    src,
    muted,
    audioSrc,
    timeMs,
    playRef,
  });

  useCompositeLoop({
    screenRef: screen, webcamRef: webcam, audioRef: audio, canvasRef: canvas,
    playRef, timeRef, onTimeRef,
    trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
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

  return (
    <div className="e-stagewrap">
      <StageToolbar />
      <div className="e-stage">
        {!src && !err && <div className="e-stage-empty"><Spin size={20} /><span>Preparing preview</span></div>}
        <canvas ref={canvas} className="e-canvas" width={1280} height={720} onClick={onCanvasClick}
          title="Click to add a zoom here" style={{ display: src ? "block" : "none", cursor: "zoom-in" }} />
        {src && (
          <video ref={screen} src={src} muted playsInline preload="auto" style={HIDDEN}
            onLoadedData={() => { setErr(null); dirtyRef.current = true; }}
            onSeeked={() => { dirtyRef.current = true; }}
            onEnded={() => { const v = screen.current; if (v && isFinite(v.duration)) onTimeRef.current(Math.round(v.duration * 1000)); }}
            onLoadedMetadata={(e) => {
              const v = e.currentTarget; const d = v.duration;
              if (isFinite(d) && d > 0) onDuration(Math.round(d * 1000));
              v.currentTime = Math.max(0, timeRef.current / 1000); // restore position across the raw->proxy swap
              if (playRef.current) v.play().catch(() => {});
            }}
            onError={(e) => setErr(MEDIA_ERR[e.currentTarget.error?.code ?? 0] || "load failed")} />
        )}
        {webcamSrc && <video ref={webcam} src={webcamSrc} muted playsInline preload="auto" style={HIDDEN}
          onLoadedData={() => { dirtyRef.current = true; }} onSeeked={() => { dirtyRef.current = true; }} />}
        {audioSrc && <audio ref={audio} src={audioSrc} preload="auto" />}
        {err && <div className="e-stage-empty" style={{ position: "absolute", inset: 0 }}>Preview unavailable - {err}</div>}
      </div>
    </div>
  );
}
