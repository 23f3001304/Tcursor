import { useEffect, useImperativeHandle, useState, type Ref } from "react";
import { motion, animate, useSpring } from "motion/react";
import { anchorPoint } from "./targets";
import { pace } from "./choreography";

const SPRING = { stiffness: 170, damping: 26 };
const SETTLE_PX = 2; // moveTo resolves once x/y are within this many px of the target
const sleep = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

export interface DirectorPointerHandle {
  /** Glides to (x, y). Resolves once the spring has settled within `SETTLE_PX`, or after
   *  `pace(distance).travelCapMs` - whichever comes first - so a long-distance move can never
   *  stall the reveal on spring overshoot. */
  moveTo(x: number, y: number): Promise<void>;
  /** A ~180ms "click": the ring pulses (18px/20% alpha -> 10px/60% alpha and back) while the
   *  glyph dips to 0.92 scale. */
  press(): Promise<void>;
  /** A fast, tween-driven (not spring) left-to-right pass across one row - used for `clear_zooms`,
   *  which has no single point to aim at. */
  sweep(fromX: number, toX: number, y: number): Promise<void>;
}

/** The AI director's fake cursor: a violet-ringed (`--e-ai`) arrow that visibly performs each
 *  edit instead of applying it invisibly. The CALLER mounts/unmounts it (`{running && <DirectorPointer .../>}`
 *  inside an `AnimatePresence`, per `DirectorOverlay.tsx`) so it fades in/out with the run itself;
 *  on mount it snaps (no glide - `.jump`, not `.set`) to whichever "wand" anchor is currently
 *  visible, so the fade-in reads as "appearing at the button", not flying in from the origin. */
export function DirectorPointer({ ref }: { ref?: Ref<DirectorPointerHandle> }) {
  const x = useSpring(0, SPRING);
  const y = useSpring(0, SPRING);
  const [pressed, setPressed] = useState(false);

  useEffect(() => {
    const p = anchorPoint("wand");
    if (p) { x.jump(p.x); y.jump(p.y); }
  }, [x, y]);

  useImperativeHandle(ref, () => ({
    moveTo: (tx, ty) => new Promise<void>((resolve) => {
      const cap = pace(Math.hypot(tx - x.get(), ty - y.get())).travelCapMs;
      let settled = false;
      const finish = () => { if (settled) return; settled = true; offX(); offY(); clearTimeout(timer); resolve(); };
      const check = () => { if (Math.abs(x.get() - tx) <= SETTLE_PX && Math.abs(y.get() - ty) <= SETTLE_PX) finish(); };
      const offX = x.on("change", check);
      const offY = y.on("change", check);
      const timer = setTimeout(finish, cap);
      x.set(tx); y.set(ty);
      check(); // already there (e.g. a zero-distance move) - resolve without waiting on "change"
    }),
    press: async () => {
      setPressed(true);
      await sleep(90);
      setPressed(false);
      await sleep(90);
    },
    sweep: async (fromX, toX, ty) => {
      x.jump(fromX); y.jump(ty);
      await animate(x, toX, { type: "tween", duration: 0.32, ease: [0.4, 0, 0.2, 1] });
    },
  }), [x, y]);

  return (
    <motion.div className="e-director-ptr" style={{ x, y }}
      initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.16 }}>
      <motion.span className="e-director-ring"
        animate={{ width: pressed ? 10 : 18, height: pressed ? 10 : 18, opacity: pressed ? 0.6 : 0.2 }}
        transition={{ type: "tween", duration: 0.09, ease: [0.4, 0, 0.2, 1] }} />
      <motion.svg className="e-director-glyph" width="20" height="22" viewBox="0 0 20 22"
        animate={{ scale: pressed ? 0.92 : 1 }} transition={{ type: "tween", duration: 0.09 }}>
        <path d="M1 1 L1 16 L5 12.3 L7.6 18.4 L10.3 17.2 L7.9 11.2 L13 11.2 Z"
          fill="var(--e-fg)" stroke="var(--e-bg)" strokeWidth="1.5" strokeLinejoin="round" />
      </motion.svg>
    </motion.div>
  );
}
