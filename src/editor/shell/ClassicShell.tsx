import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { Rail } from "./Rail";
import { EditorPanels } from "../EditorPanels";
import { PropertiesSlot, selectedClip } from "./PropertiesSlot";
import { Stage } from "../stage/Stage";
import { Transport } from "../stage/Transport";
import { Timeline } from "../timeline/Timeline";
import { Toast } from "./Toast";
import { DirectorOverlay } from "../director/DirectorOverlay";
import { outDurMs, outOf } from "../../lib/remap";
import { nextTab } from "./panelState";
import type { ShellProps } from "./slotProps";
import type { Tab } from "./panelTabs";

/** The properties sidebar's own width, in px. The aside animates to it and `.e-props-side` caps it
 *  with `max-width`, so the arrival spring's overshoot is absorbed by the layout instead of pushing
 *  the stage a few px narrower than its resting size and back. */
const SIDE_W = 360;
/** The panel column's own width, matching `.e-panel-slot`'s fixed 320px: the wrapper animates to
 *  it, the slot inside already holds it, so the panel never re-typesets while the column moves. */
const PANEL_W = 320;
/** The arrival: the width spring the owner specified, and StateSwap's frost (blur + opacity tweens,
 *  the spring on scale alone - a spring on a blur reads as flicker) on the content inside it. */
const ARRIVE = { type: "spring" as const, stiffness: 340, damping: 26 };
const FROST_IN = {
  scale: ARRIVE,
  filter: { duration: 0.26, ease: [0.22, 1, 0.36, 1] as const },
  opacity: { duration: 0.2 },
};
const LEAVE = { duration: 0.14, ease: [0.4, 0, 1, 1] as const };
const FROSTED = { opacity: 0, filter: "blur(10px)", scale: 0.97 };
const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };

/** The editor body in its classic composition plus a properties sidebar: the left icon rail and
 *  its panel slot (the rail's tab, when one is open - pressing the open tab again, or a panel's
 *  own close X, collapses that column onto the same width animation the sidebar uses, and the
 *  stage takes the width), the stage with its toasts, and on the right a column
 *  that exists only while a clip is selected - it arrives from the right edge on a width spring
 *  with its content frosting in, and collapses the same way in reverse - then the transport and
 *  the timeline underneath. Selecting a pill never replaces a panel and opening a
 *  panel never drops the selection: the two are independent, side by side (owner's call, 2026-09-13,
 *  after "clicking an effect replaces the left panel" read as bad flow). The area/workspace shell
 *  that briefly replaced this composition was vetoed and lives on `archive/m1a-shell`; everything
 *  it was built to host (the time remap's lane and readout, the background assets, the cut and
 *  speed inspectors) is here unchanged. */
export function ClassicShell(p: ShellProps) {
  const still = useReducedMotion();
  // A rail press toggles: the tab that is already open closes the column, any other opens on it
  // (`nextTab`, panelState.ts). The rail itself never moves, so there is always a way back in.
  const onTab = (t: Tab) => p.setTab((cur) => nextTab(cur, t));
  const slotProps = { ...p, onTab };
  // The sidebar EXISTS only while a clip is selected (owner's call, 2026-09-14): with nothing
  // selected the stage takes the width back rather than holding a column of empty chrome. Every
  // deselection path - Escape, an empty click on the stage or the timeline, deleting the clip -
  // already lands on `sel = null`, so they all collapse it through this one condition.
  const selected = selectedClip(p.doc, p.sel) !== null;
  return (
    <>
      <div className="e-body">
        <Rail tab={p.tab} onTab={onTab} />
        {/* The panel column exists only while a tab is open - the same "no empty chrome" rule the
            properties sidebar follows on the right, and the same width animation, so both sides of
            the stage arrive and leave on one motion language. The stage takes the freed width. */}
        <AnimatePresence initial={false}>
          {p.tab !== null && (
            <motion.div key="panel" className="e-panel-wrap"
              initial={still ? false : { width: 0 }} animate={{ width: PANEL_W }}
              exit={{ width: 0, transition: still ? { duration: 0 } : LEAVE }} transition={ARRIVE}>
              <EditorPanels folder={p.folder} doc={p.doc} tab={p.tab} setTab={p.setTab} timeMs={p.tab === "camera" ? p.timeMs : 0}
                timeMsRef={p.timeMsRef} running={p.running} exporting={p.exporting} aiError={p.aiError} aiLog={p.aiLog}
                aiProgress={p.aiProgress} onRun={p.onRun} onAutoModel={p.onAutoModel} applyOp={p.applyOp}
                saveDocSettings={p.saveDocSettings} moveMode={p.moveMode} requestMoveMode={p.onMoveMode}
                camDraftRef={p.camDraftRef} osCursorInVideo={p.osCursor} hasCursorLayer={p.cursorLyr !== null}
                addZoom={p.addZoom} addSpotlight={p.addSpotlight} addCameraMove={p.addCameraMove} />
            </motion.div>
          )}
        </AnimatePresence>
        <div className="e-stagetoast">
          <Stage folder={p.folder} src={p.srcUrl} webcamSrc={p.webcamSrc} track={p.track} layout={p.layout} layoutPresets={p.layoutPresets}
            layoutSegs={p.outDoc.layout} cameraMoves={p.outDoc.camera_moves} zooms={p.outDoc.zooms} zoomSettings={p.doc.settings.zoom}
            clicks={p.clicks} bg={p.bg} map={p.map} cursorSprites={p.cursorSpr} cursorKinds={p.cursorKnd} cursorLayer={p.cursorLyr}
            osCursorInVideo={p.osCursor} cursor={p.doc.settings.cursor} effects={p.outDoc.effects} clickfx={p.doc.settings.clickfx}
            audioSrc={p.audioUrl} muted={p.muted} volume={p.volume / 100} timeMs={p.timeMs} playing={p.playing}
            moveMode={p.moveMode} aimPoint={p.aimPoint} aimMode={p.aimMode} arrangeSeg={p.arrangeSeg} camDraftRef={p.camDraftRef}
            onTime={p.onTime} onDuration={p.onDuration} onZoomAt={p.zoomAt} onAimAt={p.aimAt} onApply={p.applyOp}
            onRetryMedia={p.retryMedia} />
          <Toast msg={p.toast} onDone={p.dismissToast} />
          <DirectorOverlay running={p.running} planning={p.aiPlanning} model={p.doc.settings.ai_model || undefined}
            pointerRef={p.pointerRef} progress={p.aiProgress} onCancel={p.onCancelRun} />
        </div>
        <AnimatePresence initial={false}>
          {selected && (
            <motion.aside key="props" className="e-props-side" aria-label="Properties"
              initial={still ? false : { width: 0 }} animate={{ width: SIDE_W }}
              exit={{ width: 0, transition: still ? { duration: 0 } : LEAVE }} transition={ARRIVE}>
              <motion.div className="e-props-frost"
                initial={still ? false : FROSTED} animate={SHOWN}
                exit={still ? {} : { ...FROSTED, transition: LEAVE }} transition={FROST_IN}>
                <PropertiesSlot p={slotProps} />
              </motion.div>
            </motion.aside>
          )}
        </AnimatePresence>
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
