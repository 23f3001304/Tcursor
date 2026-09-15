import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { Rail } from "./Rail";
import { EditorPanels } from "../EditorPanels";
import { PropertiesSlot, selectedClip } from "./PropertiesSlot";
import { ShellStage } from "./ShellStage";
import { Transport } from "../stage/transport/Transport";
import { Timeline } from "../timeline/Timeline";
import { outDurMs, outOf } from "../../shared/math/remap";
import { nextTab } from "./panelState";
import { useDensity } from "./useDensity";
import type { ShellProps } from "./slotProps";
import type { Tab } from "./PanelTabs";

const ARRIVE = { type: "spring" as const, stiffness: 340, damping: 26 };
const FROST_IN = {
  scale: ARRIVE,
  filter: { duration: 0.26, ease: [0.22, 1, 0.36, 1] as const },
  opacity: { duration: 0.2 },
};
const LEAVE = { duration: 0.14, ease: [0.4, 0, 1, 1] as const };
const FROSTED = { opacity: 0, filter: "blur(10px)", scale: 0.97 };
const SHOWN = { opacity: 1, filter: "blur(0px)", scale: 1 };

export function ClassicShell(p: ShellProps) {
  const still = useReducedMotion();
  const { panel: PANEL_W, side: SIDE_W } = useDensity();
  const onTab = (t: Tab) => p.setTab((cur) => nextTab(cur, t));
  const slotProps = { ...p, onTab };
  const selected = selectedClip(p.doc, p.sel) !== null;
  return (
    <>
      <div className="e-body">
        <Rail tab={p.tab} onTab={onTab} />
        <AnimatePresence initial={false}>
          {p.tab !== null && (
            <motion.div
              key="panel"
              className="e-panel-wrap"
              initial={still ? false : { width: 0 }}
              animate={{ width: PANEL_W }}
              exit={{ width: 0, transition: still ? { duration: 0 } : LEAVE }}
              transition={ARRIVE}
            >
              <EditorPanels
                folder={p.folder}
                doc={p.doc}
                tab={p.tab}
                setTab={p.setTab}
                timeMs={p.tab === "camera" || p.tab === "captions" ? p.timeMs : 0}
                timeMsRef={p.timeMsRef}
                running={p.running}
                exporting={p.exporting}
                aiError={p.aiError}
                aiProgress={p.aiProgress}
                onRun={p.onRun}
                onAutoModel={p.onAutoModel}
                applyOp={p.applyOp}
                reloadDoc={p.reloadDoc}
                sel={p.sel}
                onSel={p.onSel}
                onSeek={p.onSeek}
                saveDocSettings={p.saveDocSettings}
                moveMode={p.moveMode}
                requestMoveMode={p.onMoveMode}
                hasWebcam={p.hasWebcam}
                camDraftRef={p.camDraftRef}
                osCursorInVideo={p.osCursor}
                hasCursorLayer={p.cursorLyr !== null}
                addZoom={p.addZoom}
                addSpotlight={p.addSpotlight}
                addCameraMove={p.addCameraMove}
                aiRun={p.aiRun}
                aiSkipped={p.aiSkipped}
                aiApplying={p.aiApplying}
                aiPreviewId={p.aiPreviewId}
                onToggleItem={p.onToggleItem}
                onPreviewItem={p.onPreviewItem}
                onApplyRun={p.onApplyRun}
                onDiscardRun={p.onDiscardRun}
              />
            </motion.div>
          )}
        </AnimatePresence>
        <ShellStage p={p} />
        <AnimatePresence initial={false}>
          {selected && (
            <motion.aside
              key="props"
              className="e-props-side"
              aria-label="Properties"
              initial={still ? false : { width: 0 }}
              animate={{ width: SIDE_W }}
              exit={{ width: 0, transition: still ? { duration: 0 } : LEAVE }}
              transition={ARRIVE}
            >
              <motion.div
                className="e-props-frost"
                initial={still ? false : FROSTED}
                animate={SHOWN}
                exit={still ? {} : { ...FROSTED, transition: LEAVE }}
                transition={FROST_IN}
              >
                <PropertiesSlot p={slotProps} />
              </motion.div>
            </motion.aside>
          )}
        </AnimatePresence>
      </div>
      <Transport
        timeMs={p.timeMs}
        dur={p.dur}
        outTimeMs={outOf(p.map, p.timeMs)}
        outDur={outDurMs(p.map)}
        plain={p.map.plain}
        playing={p.playing}
        onPlay={p.onPlayToggle}
        onSeek={p.onSeek}
        onAddZoom={p.addZoom}
        onAutoedit={p.onRun}
        aiRunning={p.running}
        exporting={p.exporting}
        trimmed={p.trimmed}
        onTrimIn={p.onTrimIn}
        onTrimOut={p.onTrimOut}
        onResetTrim={p.onResetTrim}
        aspect={p.doc.aspect}
        onAspect={p.onAspect}
        quality={p.quality}
        onQuality={p.cycleQuality}
        muted={p.muted}
        onMute={p.onMuteToggle}
        volume={p.volume}
        onVolume={p.setVolume}
        clicks={p.clicks}
        range={p.range}
        setRange={p.setRange}
        onApply={p.applyOp}
        onDetectSilences={p.onDetectSilences}
      />
      <Timeline
        doc={p.doc}
        timeMs={p.timeMs}
        dur={p.dur}
        playing={p.playing}
        onSeek={p.onSeek}
        sel={p.sel}
        onSel={p.onSel}
        onApply={p.applyOp}
        thumbs={p.thumbs}
        waves={p.waves}
        wavesReady={p.wavesReady}
        hasWebcam={p.hasWebcam}
        layoutPresets={p.layoutPresets}
        range={p.range}
        setRange={p.setRange}
      />
    </>
  );
}
