import { AnimatePresence, motion } from "motion/react";
import type { Dispatch, RefObject, SetStateAction } from "react";
import type { EditDoc, EditOp } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import type { Tab } from "./shell/Rail";
import { AiPanel } from "./panels/AiPanel";
import { ZoomInspector } from "./inspectors/ZoomInspector";
import { EffectInspector } from "./inspectors/EffectInspector";
import { LayoutInspector } from "./inspectors/LayoutInspector";
import { CameraMoveInspector } from "./inspectors/CameraMoveInspector";
import { BackgroundPanel } from "./panels/BackgroundPanel";
import { CursorPanel } from "./panels/CursorPanel";
import { CameraPanel } from "./panels/CameraPanel";
import { CaptionsPanel } from "./panels/CaptionsPanel";
import { AudioPanel } from "./panels/AudioPanel";
import { EffectsPanel } from "./panels/EffectsPanel";

// The left-hand inspector/panel router: an inspector for the current selection, else the panel for
// the active rail tab. Split out of Editor (which was over the line limit) - it owns only the
// "which panel" switch; every edit still flows through the applyOp/saveDocSettings it is handed.
export function EditorPanels({
  doc, sel, tab, dur, setSel, setTab, timeMs, running, aiError, aiLog, onRun, applyOp, saveDocSettings,
  moveMode, requestMoveMode, camDraftRef, addZoom, addSpotlight, addCameraMove,
}: {
  doc: EditDoc;
  sel: string | null;
  tab: Tab;
  dur: number;
  setSel: Dispatch<SetStateAction<string | null>>;
  setTab: Dispatch<SetStateAction<Tab>>;
  timeMs: number;
  running: boolean;
  aiError: string | null;
  aiLog: string[];
  onRun: () => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  moveMode: boolean;
  requestMoveMode: (want: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
}) {
  const selZoom = doc.zooms.find((z) => z.id === sel) ?? null;
  const selEffect = doc.effects.find((e) => e.id === sel) ?? null;
  const selLayout = doc.layout.find((l) => l.id === sel) ?? null;
  const selCamMove = doc.camera_moves.find((m) => m.id === sel) ?? null;
  return (
    <AnimatePresence mode="popLayout" initial={false}>
      <motion.div key={sel ?? tab} className="e-panel-slot"
        initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }}
        transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
        {selZoom ? (
          <ZoomInspector zoom={selZoom} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
        ) : selEffect ? (
          <EffectInspector effect={selEffect} dur={dur} settings={doc.settings} onApply={applyOp}
            onDimCamera={(v) => saveDocSettings({ ...doc.settings, clickfx: { ...doc.settings.clickfx, spotlight_dim_camera: v } })}
            onClose={() => setSel(null)} />
        ) : selLayout ? (
          <LayoutInspector seg={selLayout} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
        ) : selCamMove ? (
          <CameraMoveInspector move={selCamMove} dur={dur} onApply={applyOp} onClose={() => setSel(null)} />
        ) : tab === "ai" ? (
          <AiPanel running={running} error={aiError} log={aiLog} onRun={onRun} model={doc.settings.ai_model}
            onChangeModel={(v) => saveDocSettings({ ...doc.settings, ai_model: v })} />
        ) : tab === "background" ? (
          <BackgroundPanel doc={doc} onSaveSettings={saveDocSettings} onClose={() => setTab("ai")} />
        ) : tab === "cursor" ? (
          <CursorPanel settings={doc.settings.cursor} onChange={(cursor) => saveDocSettings({ ...doc.settings, cursor })} onClose={() => setTab("ai")} />
        ) : tab === "camera" ? (
          <CameraPanel settings={doc.settings.appearance} onChange={(appearance) => saveDocSettings({ ...doc.settings, appearance })} onClose={() => setTab("ai")}
            doc={doc} timeMs={timeMs} applyOp={applyOp} moveMode={moveMode} onMoveModeChange={requestMoveMode} camDraftRef={camDraftRef} />
        ) : tab === "captions" ? (
          <CaptionsPanel settings={doc.settings.clickfx} onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })} onClose={() => setTab("ai")} />
        ) : tab === "audio" ? (
          <AudioPanel offsetMs={doc.settings.audio_offset_ms} onChangeOffset={(v) => saveDocSettings({ ...doc.settings, audio_offset_ms: v })}
            micVol={doc.settings.audio_mic_volume} onChangeMicVol={(v) => saveDocSettings({ ...doc.settings, audio_mic_volume: v })}
            sysVol={doc.settings.audio_sys_volume} onChangeSysVol={(v) => saveDocSettings({ ...doc.settings, audio_sys_volume: v })}
            onClose={() => setTab("ai")} />
        ) : tab === "effects" ? (
          <EffectsPanel settings={doc.settings.clickfx} onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })} onClose={() => setTab("ai")} onAddZoom={addZoom} onAddSpotlight={addSpotlight} onAddLayout={async () => { await applyOp({ op: "add_layout_seg", at_ms: Math.round(timeMs), dur_ms: 2000, layout: "camera" }); }} onAddCameraMove={addCameraMove} />
        ) : (
          <div className="e-panel"><h2 style={{ textTransform: "capitalize" }}>{tab}</h2>
            <p className="e-lede">{tab} settings land here next.</p></div>
        )}
      </motion.div>
    </AnimatePresence>
  );
}
