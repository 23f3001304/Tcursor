import { memo, useRef, useState } from "react";
import { useStagePointer } from "./useStagePointer";
import { ArrangeOverlay } from "./arrange/ArrangeOverlay";
import { CamDragHandle } from "./camera/CamDragHandle";
import { ZoomReticle } from "./ZoomReticle";
import { StageOutline } from "./StageOutline";
import { StageEmpty } from "./StageEmpty";
import { MEDIA_ERR, StageMedia } from "./StageMedia";
import { useStageEngine } from "./useStageEngine";
import type { StageProps } from "./stageProps";
import { stageFrameStyle, useFrameSize, useViewMode } from "./transport/viewMode";
import "./stage.css";

export type { StageProps } from "./stageProps";
export { stageCursor } from "./stageCursor";
export { StageMedia } from "./StageMedia";

const DEFAULT_CANVAS: [number, number] = [1280, 720];

export const Stage = memo(function Stage(p: StageProps) {
  const screen = useRef<HTMLVideoElement>(null);
  const webcam = useRef<HTMLVideoElement>(null);
  const audio = useRef<HTMLAudioElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const wrap = useRef<HTMLDivElement>(null);
  const [err, setErr] = useState<string | null>(null);
  const [canvasW, canvasH] = p.layout?.canvas ?? DEFAULT_CANVAS;

  const { arrange, arranging, dirtyRef, tOut, mapRef, layoutRef, trackRef, timeRef, playRef, onTimeRef } =
    useStageEngine(p, { screen, webcam, audio, canvas }, canvasW, canvasH);

  const { onCanvasClick, reticle, aimDrag, onReticleDown } = useStagePointer({
    canvasRef: canvas,
    screenRef: screen,
    layoutRef,
    trackRef,
    mapRef,
    timeRef,
    arranging,
    aimMode: p.aimMode,
    aimPoint: p.aimPoint,
    playing: p.playing,
    canvasW,
    canvasH,
    layout: p.layout,
    track: p.track,
    tOut,
    onZoomAt: p.onZoomAt,
    onAimAt: p.onAimAt,
  });

  const view = useViewMode();
  const [frameW, frameH] = useFrameSize(wrap);

  return (
    <div ref={wrap} className={`e-stagewrap${view === "fill" ? " fill" : ""}`}>
      <div
        className="e-stage"
        data-ui-fx="off"
        style={stageFrameStyle(view, canvasW, canvasH, frameW, frameH)}
      >
        {!p.src && !err && <StageEmpty />}
        <canvas
          ref={canvas}
          className="e-canvas"
          width={canvasW}
          height={canvasH}
          onClick={onCanvasClick}
          title={
            arranging
              ? "Drag the panel frames to arrange this layout"
              : p.aimMode
                ? "Click to aim this zoom"
                : "Click to add a zoom here"
          }
          style={{
            display: p.src ? "block" : "none",
            cursor: arranging ? "default" : p.aimMode ? "crosshair" : "zoom-in",
          }}
        />
        <StageOutline
          rect={p.outline}
          canvasW={canvasW}
          canvasH={canvasH}
          layout={p.layout}
          track={p.track}
          tOut={tOut}
        />
        {reticle && (
          <ZoomReticle
            x={reticle[0]}
            y={reticle[1]}
            aiming={p.aimMode}
            dragging={aimDrag}
            onPointerDown={p.aimMode ? onReticleDown : undefined}
          />
        )}
        {p.moveMode && !arranging && (
          <CamDragHandle
            layout={p.layout}
            layoutPresets={arrange.presets}
            layoutSegs={p.layoutSegs}
            cameraMoves={p.cameraMoves}
            timeMs={tOut}
            playing={p.playing}
            canvasW={canvasW}
            canvasH={canvasH}
            canvasRef={canvas}
            camDraftRef={p.camDraftRef}
            dirtyRef={dirtyRef}
          />
        )}
        {p.arrangeSeg && arrange.panels && (
          <ArrangeOverlay
            seg={p.arrangeSeg}
            panels={arrange.panels}
            camMoves={p.cameraMoves}
            guideX={arrange.guideX}
            guideY={arrange.guideY}
            active={arrange.active}
            onPanelDown={arrange.onPanelDown}
            onHideCam={arrange.onHideCam}
          />
        )}
        <StageMedia
          screenRef={screen}
          webcamRef={webcam}
          audioRef={audio}
          src={p.src}
          webcamSrc={p.webcamSrc}
          audioSrc={p.audioSrc}
          err={err}
          onScreenLoadedData={() => {
            setErr(null);
            dirtyRef.current = true;
          }}
          onScreenSeeked={() => {
            dirtyRef.current = true;
          }}
          onScreenEnded={() => {
            const v = screen.current;
            if (v && isFinite(v.duration)) onTimeRef.current(Math.round(v.duration * 1000));
          }}
          onScreenLoadedMetadata={(e) => {
            const v = e.currentTarget;
            const d = v.duration;
            if (isFinite(d) && d > 0) p.onDuration(Math.round(d * 1000));
            v.currentTime = Math.max(0, timeRef.current / 1000);
            if (playRef.current) v.play().catch(() => {});
          }}
          onScreenError={(e) => setErr(MEDIA_ERR[e.currentTarget.error?.code ?? 0] || "load failed")}
          onWebcamLoadedData={() => {
            dirtyRef.current = true;
          }}
          onWebcamSeeked={() => {
            dirtyRef.current = true;
          }}
          onRetry={() => {
            setErr(null);
            p.onRetryMedia();
            screen.current?.load();
          }}
        />
      </div>
    </div>
  );
});
