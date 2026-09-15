import { AnimatePresence, motion } from "motion/react";
import { IconX } from "@tabler/icons-react";
import type { CameraMove, LayoutSeg } from "../../../shared/edit";
import type { PanelRectDto } from "../../../shared/ipc";
import { VISIBLE_ALPHA, type Corner, type PanelKind, type Panels } from "./arrangeMath";

const CORNERS: Corner[] = ["tl", "tr", "bl", "br"];

const GUIDE_FADE = { duration: 0.1, ease: "easeOut" } as const;
const FADE = { initial: { opacity: 0 }, animate: { opacity: 1 }, exit: { opacity: 0 } };

export function ArrangeOverlay({
  seg,
  panels,
  camMoves,
  guideX,
  guideY,
  active,
  onPanelDown,
  onHideCam,
}: {
  seg: LayoutSeg;
  panels: Panels;
  camMoves: CameraMove[];
  guideX: number | null;
  guideY: number | null;
  active: PanelKind | null;
  onPanelDown: (e: React.PointerEvent, panel: PanelKind, corner: Corner | null) => void;
  onHideCam: () => void;
}) {
  const kfs = camMoves.filter((m) => m.t_ms >= seg.start_ms && m.t_ms < seg.end_ms);
  const canHideCam = panels.screen.alpha > VISIBLE_ALPHA;
  const frame = (kind: PanelKind, p: PanelRectDto) => (
    <div
      className={`e-apanel${active === kind ? " drag" : ""}`}
      style={{
        left: `${p.rect[0] * 100}%`,
        top: `${p.rect[1] * 100}%`,
        width: `${p.rect[2] * 100}%`,
        height: `${p.rect[3] * 100}%`,
      }}
      title={`Drag to move the ${kind === "cam" ? "webcam" : "screen"}; corners resize it`}
      onPointerDown={(e) => onPanelDown(e, kind, null)}
    >
      {CORNERS.map((c) => (
        <i key={c} className={`e-ahandle ${c}`} onPointerDown={(e) => onPanelDown(e, kind, c)} />
      ))}
      {kind === "cam" && canHideCam && (
        <button
          type="button"
          className="e-ax"
          title="Hide the webcam in this segment"
          aria-label="Hide the webcam in this segment"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={onHideCam}
        >
          <IconX size={11} />
        </button>
      )}
    </div>
  );

  return (
    <div className="e-arrange">
      {panels.screen.alpha > VISIBLE_ALPHA && frame("screen", panels.screen)}
      {panels.cam.alpha > VISIBLE_ALPHA && frame("cam", panels.cam)}
      {kfs.map((m) => (
        <i key={m.id} className="e-akf" style={{ left: `${m.x * 100}%`, top: `${m.y * 100}%` }} />
      ))}
      <AnimatePresence>
        {guideX !== null && (
          <motion.i
            key="gx"
            className="e-aguide x"
            style={{ left: `${guideX * 100}%` }}
            {...FADE}
            transition={GUIDE_FADE}
          />
        )}
        {guideY !== null && (
          <motion.i
            key="gy"
            className="e-aguide y"
            style={{ top: `${guideY * 100}%` }}
            {...FADE}
            transition={GUIDE_FADE}
          />
        )}
      </AnimatePresence>
      {kfs.length > 0 && <p className="e-ahint">Keyframes refine the webcam inside this segment</p>}
    </div>
  );
}
