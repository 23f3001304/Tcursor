import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { ZoomInspector } from "../inspectors/ZoomInspector";
import { EffectInspector } from "../inspectors/EffectInspector";
import { LayoutInspector } from "../inspectors/LayoutInspector";
import { CameraMoveInspector } from "../inspectors/CameraMoveInspector";
import { CutInspector } from "../inspectors/CutInspector";
import { SpeedInspector } from "../inspectors/SpeedInspector";
import type { SlotProps } from "./slotProps";
import "../inspectors/inspectors.css";

/** The one content-swap tween (design/premium-pass D6): the same beat `EditorPanels` swaps tabs on. */
const SWAP_TWEEN = { type: "tween" as const, duration: 0.16, ease: [0.4, 0, 0.2, 1] as const };

/** The selected clip's inspector: the six inspectors, routed by the current selection. `ClassicShell`
 *  shows it in the one panel slot while something is selected and the rail's tab otherwise, so
 *  selecting a pill replaces the panel and opening a panel drops the selection - one slot, one thing.
 *  With a stale selection that matches nothing it says what a click would open. */
export function PropertiesSlot({ p }: { p: SlotProps }) {
  const still = useReducedMotion();
  const { doc, sel } = p;
  const zoom = doc.zooms.find((z) => z.id === sel) ?? null;
  const effect = doc.effects.find((e) => e.id === sel) ?? null;
  const layout = doc.layout.find((l) => l.id === sel) ?? null;
  const move = doc.camera_moves.find((m) => m.id === sel) ?? null;
  const cut = doc.cuts.find((c) => c.id === sel) ?? null;
  const speed = doc.speed.find((s) => s.id === sel) ?? null;
  const close = () => p.setSel(null);
  return (
    <AnimatePresence mode="popLayout" initial={false}>
      <motion.div key={sel ?? "none"} className="e-panel-slot"
        initial={still ? false : { opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }}
        exit={still ? { opacity: 0 } : { opacity: 0, x: -8 }} transition={still ? { duration: 0 } : SWAP_TWEEN}>
        {zoom ? (
          <ZoomInspector zoom={zoom} dur={p.dur} onApply={p.applyOp} onClose={close} aimMode={p.aimMode}
            moveMode={p.moveMode} onAimMode={p.setAimOn} timeMsRef={p.timeMsRef} onSeek={p.onSeek} />
        ) : effect ? (
          <EffectInspector effect={effect} dur={p.dur} settings={doc.settings} onApply={p.applyOp} onClose={close}
            onDimCamera={(v) => p.saveDocSettings({ ...doc.settings, clickfx: { ...doc.settings.clickfx, spotlight_dim_camera: v } })} />
        ) : layout ? (
          <LayoutInspector seg={layout} dur={p.dur} onApply={p.applyOp} onClose={close} presets={p.layoutPresets}
            arrangeOn={p.arrangeOn} onArrange={p.onArrange} />
        ) : move ? (
          <CameraMoveInspector move={move} dur={p.dur} onApply={p.applyOp} onClose={close} />
        ) : cut ? (
          <CutInspector cut={cut} dur={p.dur} onApply={p.applyOp} onClose={close} />
        ) : speed ? (
          <SpeedInspector speed={speed} dur={p.dur} onApply={p.applyOp} onClose={close} />
        ) : (
          <p className="e-insp-empty">Select a zoom, layout, camera move, effect, cut or speed span on the timeline.</p>
        )}
      </motion.div>
    </AnimatePresence>
  );
}
