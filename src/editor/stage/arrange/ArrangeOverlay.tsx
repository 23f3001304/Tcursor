import { AnimatePresence, motion } from "motion/react";
import { IconX } from "@tabler/icons-react";
import type { CameraMove, LayoutSeg } from "../../../lib/edit";
import type { PanelRectDto } from "../../../lib/ipc";
import { VISIBLE_ALPHA, type Corner, type PanelKind, type Panels } from "./arrangeMath";

const CORNERS: Corner[] = ["tl", "tr", "bl", "br"];
// Hoisted (the PLAY_SPRING/PRESS_SPRING convention): a guide is a hint, so it crossfades rather
// than springs - the brief's 100ms fade.
const GUIDE_FADE = { duration: 0.1, ease: "easeOut" } as const;
const FADE = { initial: { opacity: 0 }, animate: { opacity: 1 }, exit: { opacity: 0 } };

/** Arrange mode's interactive layer: both panels of the selected layout segment framed over the
 *  live composite, draggable by their body (move) and their corners (resize about the opposite
 *  corner). Positioned as a % of `.e-stage`, which is exactly the canvas' own displayed rect - the
 *  same trick `CamDragHandle` uses, and the reason hit-testing is the DOM's job here rather than a
 *  second copy of the panel geometry in JS.
 *
 *  Only the CAM carries a hide affordance: hiding the screen from the stage could leave a blank
 *  frame with nothing left to grab, so that stays an inspector-only action. */
export function ArrangeOverlay({ seg, panels, camMoves, guideX, guideY, active, onPanelDown, onHideCam }: {
  seg: LayoutSeg; panels: Panels; camMoves: CameraMove[];
  guideX: number | null; guideY: number | null; active: PanelKind | null;
  onPanelDown: (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => void;
  onHideCam: () => void;
}) {
  // Keyframes INSIDE this segment still own the webcam pose while they run (T27 semantics,
  // unchanged by arranging) - shown dimmed so the two systems don't read as one.
  const kfs = camMoves.filter((m) => m.t_ms >= seg.start_ms && m.t_ms < seg.end_ms);
  // The x is offered only when hiding the cam would still leave the screen shown. Otherwise the
  // write is the "hide both panels" case Rust rejects, and a rejected op still resolves - so the
  // button would look like it worked while leaving a phantom undo step behind (reachable today
  // via the shipped Camera-only preset, whose screen panel is hidden).
  const canHideCam = panels.screen.alpha > VISIBLE_ALPHA;
  const frame = (kind: PanelKind, p: PanelRectDto) => (
    <div className={`e-apanel${active === kind ? " drag" : ""}`}
      style={{ left: `${p.rect[0] * 100}%`, top: `${p.rect[1] * 100}%`, width: `${p.rect[2] * 100}%`, height: `${p.rect[3] * 100}%` }}
      title={`Drag to move the ${kind === "cam" ? "webcam" : "screen"}; corners resize it`}
      onPointerDown={(e) => onPanelDown(e, kind, null)}>
      {CORNERS.map((c) => <i key={c} className={`e-ahandle ${c}`} onPointerDown={(e) => onPanelDown(e, kind, c)} />)}
      {kind === "cam" && canHideCam && (
        <button type="button" className="e-ax" title="Hide the webcam in this segment" aria-label="Hide the webcam in this segment"
          onPointerDown={(e) => e.stopPropagation()} onClick={onHideCam}><IconX size={11} /></button>
      )}
    </div>
  );

  return (
    <div className="e-arrange">
      {panels.screen.alpha > VISIBLE_ALPHA && frame("screen", panels.screen)}
      {panels.cam.alpha > VISIBLE_ALPHA && frame("cam", panels.cam)}
      {kfs.map((m) => <i key={m.id} className="e-akf" style={{ left: `${m.x * 100}%`, top: `${m.y * 100}%` }} />)}
      <AnimatePresence>
        {guideX !== null && <motion.i key="gx" className="e-aguide x" style={{ left: `${guideX * 100}%` }} {...FADE} transition={GUIDE_FADE} />}
        {guideY !== null && <motion.i key="gy" className="e-aguide y" style={{ top: `${guideY * 100}%` }} {...FADE} transition={GUIDE_FADE} />}
      </AnimatePresence>
      {kfs.length > 0 && <p className="e-ahint">Keyframes refine the webcam inside this segment</p>}
    </div>
  );
}
