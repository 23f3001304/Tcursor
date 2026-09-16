import { AnimatePresence, motion } from "motion/react";
import type { MaskPx } from "./maskPreview";
import type { MaskHandle } from "./useMaskDrag";

const HANDLES: MaskHandle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
const GUIDE_FADE = { duration: 0.1, ease: "easeOut" } as const;
const FADE = { initial: { opacity: 0 }, animate: { opacity: 1 }, exit: { opacity: 0 } };

export function MaskOverlay({
  box,
  canvasW,
  canvasH,
  guideX,
  guideY,
  onHandleDown,
}: {
  box: MaskPx;
  canvasW: number;
  canvasH: number;
  guideX: number | null;
  guideY: number | null;
  onHandleDown: (e: React.PointerEvent, handle: MaskHandle) => void;
}) {
  const pct = (v: number, of: number) => `${(v / Math.max(1, of)) * 100}%`;
  return (
    <div className="e-maskwrap">
      <div
        className="e-maskbox"
        style={{
          left: pct(box.mn[0], canvasW),
          top: pct(box.mn[1], canvasH),
          width: pct(box.mx[0] - box.mn[0], canvasW),
          height: pct(box.mx[1] - box.mn[1], canvasH),
        }}
        title="Drag to move this mask; the handles resize it"
        onPointerDown={(e) => onHandleDown(e, "move")}
      >
        {HANDLES.map((h) => (
          <i key={h} className={`e-maskhandle ${h}`} onPointerDown={(e) => onHandleDown(e, h)} />
        ))}
      </div>
      <AnimatePresence>
        {guideX !== null && (
          <motion.i
            key="mx"
            className="e-aguide x"
            style={{ left: `${guideX * 100}%` }}
            {...FADE}
            transition={GUIDE_FADE}
          />
        )}
        {guideY !== null && (
          <motion.i
            key="my"
            className="e-aguide y"
            style={{ top: `${guideY * 100}%` }}
            {...FADE}
            transition={GUIDE_FADE}
          />
        )}
      </AnimatePresence>
    </div>
  );
}
