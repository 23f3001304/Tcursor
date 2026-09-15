import { useEffect, useState, type RefObject } from "react";
import { useSpring, type MotionValue } from "motion/react";
import { useReducedMotion } from "../../shared/wave/ui/useReducedMotion";
import { interfaceEffectsOn, onInterfaceEffectsChange } from "./effectsFlag";

export const MAGNET_SPRING = { stiffness: 300, damping: 20 } as const;

export const MAGNET_RADIUS = 28;
export const MAGNET_STRENGTH = 0.25;

export interface MagneticBox {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export function magneticPull(
  box: MagneticBox,
  px: number,
  py: number,
  radius: number,
  strength: number,
): [number, number] {
  const gapX = px < box.left ? box.left - px : px > box.right ? px - box.right : 0;
  const gapY = py < box.top ? box.top - py : py > box.bottom ? py - box.bottom : 0;
  if (Math.hypot(gapX, gapY) > radius) return [0, 0];
  return [(px - (box.left + box.right) / 2) * strength, (py - (box.top + box.bottom) / 2) * strength];
}

export function useMagnetic(
  ref: RefObject<HTMLElement | null>,
  radius: number = MAGNET_RADIUS,
  strength: number = MAGNET_STRENGTH,
): { x: MotionValue<number>; y: MotionValue<number> } {
  const x = useSpring(0, MAGNET_SPRING);
  const y = useSpring(0, MAGNET_SPRING);
  const reduced = useReducedMotion();
  const [on, setOn] = useState(interfaceEffectsOn);
  useEffect(() => onInterfaceEffectsChange(setOn), []);

  useEffect(() => {
    if (!on || reduced || strength === 0) {
      x.set(0);
      y.set(0);
      return;
    }
    let raf = 0,
      px = 0,
      py = 0;
    const apply = () => {
      raf = 0;
      const el = ref.current;
      if (!el) return;
      const [dx, dy] = magneticPull(el.getBoundingClientRect(), px, py, radius, strength);
      x.set(dx);
      y.set(dy);
    };
    const onMove = (e: PointerEvent) => {
      px = e.clientX;
      py = e.clientY;
      if (!raf) raf = requestAnimationFrame(apply);
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onMove);
      if (raf) cancelAnimationFrame(raf);
      x.set(0);
      y.set(0);
    };
  }, [on, reduced, radius, strength, ref, x, y]);

  return { x, y };
}
