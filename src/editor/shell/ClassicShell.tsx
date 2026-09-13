import { Rail } from "./Rail";
import { EditorPanels } from "../EditorPanels";
import { PropertiesSlot } from "./PropertiesSlot";
import { Stage } from "../stage/Stage";
import { Transport } from "../stage/Transport";
import { Timeline } from "../timeline/Timeline";
import { Toast } from "./Toast";
import { DirectorOverlay } from "../director/DirectorOverlay";
import { outDurMs, outOf } from "../../lib/remap";
import type { ShellProps } from "./slotProps";
import type { Tab } from "./panelTabs";

/** The editor body in its classic composition plus a properties sidebar: the left icon rail and
 *  its panel slot (always the rail's tab), the stage with its toasts, and on the right a fixed
 *  column with the selected clip's inspector (or one quiet sentence when nothing is selected), then
 *  the transport and the timeline underneath. Selecting a pill never replaces a panel and opening a
 *  panel never drops the selection: the two are independent, side by side (owner's call, 2026-09-13,
 *  after "clicking an effect replaces the left panel" read as bad flow). The area/workspace shell
 *  that briefly replaced this composition was vetoed and lives on `archive/m1a-shell`; everything
 *  it was built to host (the time remap's lane and readout, the background assets, the cut and
 *  speed inspectors) is here unchanged. */
export function ClassicShell(p: ShellProps) {
  const onTab = (t: Tab) => p.setTab(t);
  const slotProps = { ...p, onTab };
  return (
    <>
      <div className="e-body">
        <Rail tab={p.tab} onTab={onTab} />
        <EditorPanels folder={p.folder} doc={p.doc} tab={p.tab} setTab={p.setTab} timeMs={p.tab === "camera" ? p.timeMs : 0}
          timeMsRef={p.timeMsRef} running={p.running} exporting={p.exporting} aiError={p.aiError} aiLog={p.aiLog}
          aiProgress={p.aiProgress} onRun={p.onRun} onAutoModel={p.onAutoModel} applyOp={p.applyOp}
          saveDocSettings={p.saveDocSettings} moveMode={p.moveMode} requestMoveMode={p.onMoveMode}
          camDraftRef={p.camDraftRef} osCursorInVideo={p.osCursor} hasCursorLayer={p.cursorLyr !== null}
          addZoom={p.addZoom} addSpotlight={p.addSpotlight} addCameraMove={p.addCameraMove} />
        <div className="e-stagetoast">
          <Stage src={p.srcUrl} webcamSrc={p.webcamSrc} track={p.track} layout={p.layout} layoutPresets={p.layoutPresets}
            layoutSegs={p.outDoc.layout} cameraMoves={p.outDoc.camera_moves} zooms={p.outDoc.zooms} zoomSettings={p.doc.settings.zoom}
            clicks={p.clicks} bg={p.bg} map={p.map} cursorSprites={p.cursorSpr} cursorKinds={p.cursorKnd} cursorLayer={p.cursorLyr}
            osCursorInVideo={p.osCursor} cursor={p.doc.settings.cursor} effects={p.outDoc.effects} clickfx={p.doc.settings.clickfx}
            audioSrc={p.audioUrl} muted={p.muted} volume={p.volume / 100} timeMs={p.timeMs} playing={p.playing}
            moveMode={p.moveMode} aimPoint={p.aimPoint} aimMode={p.aimMode} arrangeSeg={p.arrangeSeg} camDraftRef={p.camDraftRef}
            tab={p.tab} onTab={onTab} aspect={p.doc.aspect} onAspect={p.onAspect} aspectLocked={p.aspectLocked}
            onTime={p.onTime} onDuration={p.onDuration} onZoomAt={p.zoomAt} onAimAt={p.aimAt} onApply={p.applyOp}
            onRetryMedia={p.retryMedia} />
          <Toast msg={p.toast} onDone={p.dismissToast} />
          <DirectorOverlay running={p.running} planning={p.aiPlanning} model={p.doc.settings.ai_model || undefined}
            pointerRef={p.pointerRef} progress={p.aiProgress} onCancel={p.onCancelRun} />
        </div>
        <aside className="e-props-side" aria-label="Properties"><PropertiesSlot p={slotProps} /></aside>
      </div>
      <Transport timeMs={p.timeMs} dur={p.dur} outTimeMs={outOf(p.map, p.timeMs)} outDur={outDurMs(p.map)} plain={p.map.plain} playing={p.playing}
        onPlay={p.onPlayToggle} onSeek={p.onSeek} onAddZoom={p.addZoom} onAutoedit={p.onRun} aiRunning={p.running} exporting={p.exporting}
        trimmed={p.trimmed} onTrimIn={p.onTrimIn} onTrimOut={p.onTrimOut} onResetTrim={p.onResetTrim} aspect={p.doc.aspect}
        onAspect={p.onAspect} quality={p.quality} onQuality={p.cycleQuality} muted={p.muted} onMute={p.onMuteToggle}
        volume={p.volume} onVolume={p.setVolume} clicks={p.clicks} range={p.range} setRange={p.setRange} onApply={p.applyOp}
        onDetectSilences={p.onDetectSilences} />
      <Timeline doc={p.doc} timeMs={p.timeMs} dur={p.dur} playing={p.playing} onSeek={p.onSeek} sel={p.sel}
        onSel={p.onSel} onApply={p.applyOp} thumbs={p.thumbs} waves={p.waves} wavesReady={p.wavesReady}
        hasWebcam={p.hasWebcam} layoutPresets={p.layoutPresets} range={p.range} setRange={p.setRange} />
    </>
  );
}
