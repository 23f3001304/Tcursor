import { memo } from "react";
import { AnimatePresence, motion } from "motion/react";
import type { EditorPanelsProps } from "./editorPanelsProps";
import { AiPanel } from "./panels/AiPanel";
import { BackgroundPanel } from "./panels/background/BackgroundPanel";
import { CursorPanel } from "./panels/cursor/CursorPanel";
import { CameraPanel } from "./panels/camera/CameraPanel";
import { LayoutsPanel } from "./panels/layout/LayoutsPanel";
import { HotkeysPanel } from "./panels/HotkeysPanel";
import { CaptionsPanel } from "./panels/captions/CaptionsPanel";
import { AudioPanel } from "./panels/AudioPanel";
import { EffectsPanel } from "./panels/EffectsPanel";

function assertNever(x: never): never {
  throw new Error(`EditorPanels: unhandled tab ${JSON.stringify(x)}`);
}

export const EditorPanels = memo(function EditorPanels({
  folder,
  doc,
  tab,
  setTab,
  timeMs,
  timeMsRef,
  running,
  exporting,
  aiError,
  aiProgress,
  onRun,
  onAutoModel,
  applyOp,
  saveDocSettings,
  reloadDoc,
  sel,
  onSel,
  onSeek,
  moveMode,
  requestMoveMode,
  camDraftRef,
  addZoom,
  addSpotlight,
  addMask,
  addCameraMove,
  addText,
  osCursorInVideo,
  hasCursorLayer,
  hasWebcam,
  aiRun,
  aiSkipped,
  aiApplying,
  aiPreviewId,
  onToggleItem,
  onPreviewItem,
  onApplyRun,
  onDiscardRun,
}: EditorPanelsProps) {
  const collapse = () => setTab(null);
  return (
    <AnimatePresence mode="popLayout" initial={false}>
      <motion.div
        key={tab}
        className="e-panel-slot"
        initial={{ opacity: 0, x: -8 }}
        animate={{ opacity: 1, x: 0 }}
        exit={{ opacity: 0, x: -8 }}
        transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
      >
        {tab === "ai" ? (
          <AiPanel
            running={running}
            exporting={exporting}
            error={aiError}
            progress={aiProgress}
            onRun={onRun}
            model={doc.settings.ai_model}
            onChangeModel={(v) => saveDocSettings({ ...doc.settings, ai_model: v })}
            onAutoModel={onAutoModel}
            onClose={collapse}
            run={aiRun}
            skipped={aiSkipped}
            applying={aiApplying}
            previewId={aiPreviewId}
            onToggleItem={onToggleItem}
            onPreviewItem={onPreviewItem}
            onApply={onApplyRun}
            onDiscard={onDiscardRun}
          />
        ) : tab === "background" ? (
          <BackgroundPanel folder={folder} doc={doc} onSaveSettings={saveDocSettings} onClose={collapse} />
        ) : tab === "cursor" ? (
          <CursorPanel
            settings={doc.settings.cursor}
            osCursorInVideo={osCursorInVideo}
            hasCursorLayer={hasCursorLayer}
            onChange={(cursor) => saveDocSettings({ ...doc.settings, cursor })}
            onClose={collapse}
          />
        ) : tab === "camera" ? (
          <CameraPanel
            settings={doc.settings.appearance}
            onClose={collapse}
            doc={doc}
            timeMs={timeMs}
            applyOp={applyOp}
            moveMode={moveMode}
            onMoveModeChange={requestMoveMode}
            camDraftRef={camDraftRef}
            hasWebcam={hasWebcam}
          />
        ) : tab === "layouts" ? (
          <LayoutsPanel doc={doc} timeMsRef={timeMsRef} onSaveSettings={saveDocSettings} onClose={collapse} />
        ) : tab === "captions" ? (
          <CaptionsPanel
            folder={folder}
            doc={doc}
            applyOp={applyOp}
            saveDocSettings={saveDocSettings}
            reloadDoc={reloadDoc}
            timeMs={timeMs}
            sel={sel}
            onSeek={onSeek}
            onSel={onSel}
            onClose={collapse}
          />
        ) : tab === "hotkeys" ? (
          <HotkeysPanel
            settings={doc.settings.clickfx}
            onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })}
            onClose={collapse}
          />
        ) : tab === "audio" ? (
          <AudioPanel
            offsetMs={doc.settings.audio_offset_ms}
            onChangeOffset={(v) => saveDocSettings({ ...doc.settings, audio_offset_ms: v })}
            micVol={doc.settings.audio_mic_volume}
            onChangeMicVol={(v) => saveDocSettings({ ...doc.settings, audio_mic_volume: v })}
            sysVol={doc.settings.audio_sys_volume}
            onChangeSysVol={(v) => saveDocSettings({ ...doc.settings, audio_sys_volume: v })}
            onClose={collapse}
          />
        ) : tab === "effects" ? (
          <EffectsPanel
            settings={doc.settings.clickfx}
            onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })}
            onClose={collapse}
            onAddZoom={addZoom}
            onAddSpotlight={addSpotlight}
            onAddMask={addMask}
            onAddLayout={async () => {
              await applyOp({
                op: "add_layout_seg",
                at_ms: Math.round(timeMsRef.current),
                dur_ms: 2000,
                layout: "camera",
              });
            }}
            onAddCameraMove={addCameraMove}
            onAddText={addText}
          />
        ) : (
          assertNever(tab)
        )}
      </motion.div>
    </AnimatePresence>
  );
});
