import { useEffect, useImperativeHandle, useState, type Ref } from "react";
import { motion, useSpring } from "motion/react";
import { anchorPoint } from "./targets";
import { pace } from "./choreography";

const SPRING = { stiffness: 170, damping: 26 };
const SETTLE_PX = 2;
const sleep = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

export interface DirectorPointerHandle {
  moveTo(x: number, y: number): Promise<void>;
  press(): Promise<void>;
}

export function DirectorPointer({ ref }: { ref?: Ref<DirectorPointerHandle> }) {
  const x = useSpring(0, SPRING);
  const y = useSpring(0, SPRING);
  const [pressed, setPressed] = useState(false);

  useEffect(() => {
    const p = anchorPoint("wand");
    if (p) {
      x.jump(p.x);
      y.jump(p.y);
    }
  }, [x, y]);

  useImperativeHandle(
    ref,
    () => ({
      moveTo: (tx, ty) =>
        new Promise<void>((resolve) => {
          const cap = pace(Math.hypot(tx - x.get(), ty - y.get())).travelCapMs;
          let settled = false;
          const finish = () => {
            if (settled) return;
            settled = true;
            offX();
            offY();
            clearTimeout(timer);
            resolve();
          };
          const check = () => {
            if (Math.abs(x.get() - tx) <= SETTLE_PX && Math.abs(y.get() - ty) <= SETTLE_PX) finish();
          };
          const offX = x.on("change", check);
          const offY = y.on("change", check);
          const timer = setTimeout(finish, cap);
          x.set(tx);
          y.set(ty);
          check();
        }),
      press: async () => {
        setPressed(true);
        await sleep(90);
        setPressed(false);
        await sleep(90);
      },
    }),
    [x, y],
  );

  return (
    <motion.div
      className="e-director-ptr"
      style={{ x, y }}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.16 }}
    >
      <motion.span
        className="e-director-ring"
        animate={{ width: pressed ? 10 : 18, height: pressed ? 10 : 18, opacity: pressed ? 0.6 : 0.2 }}
        transition={{ type: "tween", duration: 0.09, ease: [0.4, 0, 0.2, 1] }}
      />
      <motion.svg
        className="e-director-glyph"
        width="20"
        height="22"
        viewBox="0 0 20 22"
        animate={{ scale: pressed ? 0.92 : 1 }}
        transition={{ type: "tween", duration: 0.09 }}
      >
        <path
          d="M1 1 L1 16 L5 12.3 L7.6 18.4 L10.3 17.2 L7.9 11.2 L13 11.2 Z"
          fill="var(--e-fg)"
          stroke="var(--e-bg)"
          strokeWidth="1.5"
          strokeLinejoin="round"
        />
      </motion.svg>
    </motion.div>
  );
}
