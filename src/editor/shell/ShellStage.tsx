import { Stage } from "../stage/Stage";
import { Toast } from "./dialogs/Toast";
import { DirectorOverlay } from "../director/DirectorOverlay";
import type { ShellProps } from "./slotProps";

export function ShellStage({ p }: { p: ShellProps }) {
  return (
    <div className="e-stagetoast">
      <Stage
        folder={p.folder}
        src={p.srcUrl}
        webcamSrc={p.webcamSrc}
        track={p.track}
        layout={p.layout}
        layoutPresets={p.layoutPresets}
        layoutSegs={p.outDoc.layout}
        cameraMoves={p.outDoc.camera_moves}
        zooms={p.outDoc.zooms}
        zoomSettings={p.doc.settings.zoom}
        clicks={p.clicks}
        bg={p.bg}
        map={p.map}
        clips={p.doc.clips}
        motionEasing={p.doc.settings.motion.easing}
        cursorSprites={p.cursorSpr}
        cursorKinds={p.cursorKnd}
        cursorLayer={p.cursorLyr}
        osCursorInVideo={p.osCursor}
        cursor={p.doc.settings.cursor}
        effects={p.outDoc.effects}
        clickfx={p.doc.settings.clickfx}
        grade={p.doc.settings.grade}
        captions={p.outDoc.captions}
        texts={p.outDoc.texts}
        capStyle={p.doc.settings.captions}
        accent={p.doc.settings.ui.accent}
        audioSrc={p.audioUrl}
        muted={p.muted}
        volume={p.volume / 100}
        timeMs={p.timeMs}
        playing={p.playing}
        moveMode={p.moveMode}
        aimPoint={p.aimPoint}
        aimMode={p.aimMode}
        arrangeSeg={p.arrangeSeg}
        sel={p.sel}
        outline={p.stageOutline}
        camDraftRef={p.camDraftRef}
        onTime={p.onTime}
        onSeek={p.onSeek}
        onDuration={p.onDuration}
        onZoomAt={p.zoomAt}
        onAimAt={p.aimAt}
        onApply={p.applyOp}
        onRetryMedia={p.retryMedia}
      />
      <Toast msg={p.toast} onDone={p.dismissToast} />
      <DirectorOverlay
        running={p.running}
        planning={p.aiPlanning}
        model={p.doc.settings.ai_model || undefined}
        pointerRef={p.pointerRef}
        progress={p.aiProgress}
        onCancel={p.onCancelRun}
      />
    </div>
  );
}
