import { memo } from "react";
import { AnimatePresence, motion } from "motion/react";
import type { Dispatch, RefObject, SetStateAction } from "react";
import type { EditDoc, EditOp } from "../lib/edit";
import type { CamPose } from "./stage/cameraMoves";
import type { Tab } from "./shell/panelTabs";
import { AiPanel } from "./panels/AiPanel";
import { BackgroundPanel } from "./panels/BackgroundPanel";
import { CursorPanel } from "./panels/CursorPanel";
import { CameraPanel } from "./panels/CameraPanel";
import { CaptionsPanel } from "./panels/CaptionsPanel";
import { AudioPanel } from "./panels/AudioPanel";
import { EffectsPanel } from "./panels/EffectsPanel";

// `tab` is a closed 7-member union (`Tab`, src/editor/shell/panelTabs.tsx); the routing below
// is an exhaustive if/else over all 7. This makes that exhaustiveness a COMPILE-time guarantee
// instead of a runtime fallback stub: a future Tab variant with no matching branch fails typecheck
// here (assertNever(tab) requires `tab` to have narrowed to `never`) rather than silently landing
// on a dead "settings land here next" placeholder that could never actually render.
function assertNever(x: never): never {
  throw new Error(`EditorPanels: unhandled tab ${JSON.stringify(x)}`);
}

// The panel slot's body: one panel per tab, and nothing else. The selection branch lives in
// `shell/PropertiesSlot.tsx`; `ClassicShell` shows THAT in this same slot while something is selected
// and this router otherwise, so selecting a pill on the timeline replaces the panel (one slot, one thing).
//
// `React.memo`'d (render hygiene pass) - only `CameraPanel` (tab === "camera") needs a LIVE
// playhead, so `EditorSlot` gates the `timeMs` prop to a constant `0` on every other tab instead of
// the real ticking value, letting memo actually skip a re-render on a tick while some other panel
// is showing. The "add layout segment at the playhead" button (`EffectsPanel`) still needs the TRUE
// current time at click time regardless of which panel is showing - `timeMsRef` (a ref, so it never
// defeats memo on its own) supplies that.
export const EditorPanels = memo(function EditorPanels({
  folder, doc, tab, setTab, timeMs, timeMsRef, running, exporting, aiError, aiLog, aiProgress, onRun, onAutoModel, applyOp, saveDocSettings,
  moveMode, requestMoveMode, camDraftRef, addZoom, addSpotlight, addCameraMove, osCursorInVideo, hasCursorLayer,
}: {
  /** The recording's folder - `BackgroundPanel` imports an asset into it. */
  folder: string;
  doc: EditDoc;
  tab: Tab;
  setTab: Dispatch<SetStateAction<Tab>>;
  /** `EditorSlot` passes the real playhead only while `CameraPanel` is showing, else a constant
   *  `0` - see the component doc comment above. */
  timeMs: number;
  timeMsRef: RefObject<number>;
  running: boolean;
  /** Locks the AI Director's run button too (`AiPanel`) - matches `Transport`'s wand and its own
   *  play/trim/aspect `locked` gate (L3: mutating edit.json mid-export would silently diverge the
   *  preview/doc from the file the exporter is rendering from its own snapshot). */
  exporting: boolean;
  aiError: string | null;
  aiLog: string[];
  aiProgress: { step: number; total: number } | null;
  onRun: () => void;
  onAutoModel: (v: string) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  moveMode: boolean;
  requestMoveMode: (want: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
  osCursorInVideo: boolean;
  /** Whether this recording carries a captured OS-cursor layer - gates `CursorPanel`'s
   *  "re-created from the recorded path" hint (see its prop doc). */
  hasCursorLayer: boolean;
}) {
  return (
    <AnimatePresence mode="popLayout" initial={false}>
      <motion.div key={tab} className="e-panel-slot"
        initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -8 }}
        transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
        {tab === "ai" ? (
          <AiPanel running={running} exporting={exporting} error={aiError} log={aiLog} progress={aiProgress} onRun={onRun} model={doc.settings.ai_model}
            onChangeModel={(v) => saveDocSettings({ ...doc.settings, ai_model: v })} onAutoModel={onAutoModel} onClose={() => setTab("ai")} />
        ) : tab === "background" ? (
          <BackgroundPanel folder={folder} doc={doc} onSaveSettings={saveDocSettings} onClose={() => setTab("ai")} />
        ) : tab === "cursor" ? (
          <CursorPanel settings={doc.settings.cursor} osCursorInVideo={osCursorInVideo} hasCursorLayer={hasCursorLayer} onChange={(cursor) => saveDocSettings({ ...doc.settings, cursor })} onClose={() => setTab("ai")} />
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
          <EffectsPanel settings={doc.settings.clickfx} onChange={(clickfx) => saveDocSettings({ ...doc.settings, clickfx })} onClose={() => setTab("ai")} onAddZoom={addZoom} onAddSpotlight={addSpotlight} onAddLayout={async () => { await applyOp({ op: "add_layout_seg", at_ms: Math.round(timeMsRef.current), dur_ms: 2000, layout: "camera" }); }} onAddCameraMove={addCameraMove} />
        ) : (
          assertNever(tab)
        )}
      </motion.div>
    </AnimatePresence>
  );
});
