import { useEffect, useRef, useState } from "react";
import { debounce } from "../util/debounce";
import { shouldClearOverride } from "../util/overrideClear";
import { attachPointerGesture } from "./pointerGesture";
import { pastDragThreshold } from "../util/dragThreshold";

const AIM_DEBOUNCE_MS = 80;

const aimPointEq = (a: [number, number] | null, b: [number, number] | null) =>
  (a === null && b === null) || (!!a && !!b && Math.abs(a[0] - b[0]) < 1e-9 && Math.abs(a[1] - b[1]) < 1e-9);

export function useReticleDrag(
  aimPoint: [number, number] | null,
  onAimAt: (x: number, y: number) => void,
  targetUnderPointer: (clientX: number, clientY: number) => [number, number] | null,
) {
  const [liveAim, setLiveAim] = useState<[number, number] | null>(null);
  const [aimDrag, setAimDrag] = useState(false);
  const settledAimRef = useRef<[number, number] | null>(aimPoint);
  const onAimAtRef = useRef(onAimAt);
  onAimAtRef.current = onAimAt;
  const debouncedRef = useRef<ReturnType<typeof debounce<[number, number]>> | null>(null);
  if (!debouncedRef.current)
    debouncedRef.current = debounce((x: number, y: number) => onAimAtRef.current(x, y), AIM_DEBOUNCE_MS);
  const detachRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    if (shouldClearOverride(aimDrag, liveAim !== null, aimPoint, settledAimRef.current, aimPointEq))
      setLiveAim(null);
  }, [aimPoint, aimDrag, liveAim]);
  useEffect(() => () => debouncedRef.current?.flush(), []);
  useEffect(() => () => detachRef.current?.(), []);

  const onReticleDown = (e: React.PointerEvent) => {
    e.stopPropagation();
    setAimDrag(true);
    settledAimRef.current = aimPoint;
    const start = { x: e.clientX, y: e.clientY };
    let moved = false;
    detachRef.current = attachPointerGesture(
      (ev) => {
        if (!moved) {
          if (!pastDragThreshold(ev.clientX - start.x, ev.clientY - start.y)) return;
          moved = true;
        }
        const t = targetUnderPointer(ev.clientX, ev.clientY);
        if (t) {
          setLiveAim(t);
          debouncedRef.current!(t[0], t[1]);
        }
      },
      () => {
        detachRef.current = null;
        setAimDrag(false);
        debouncedRef.current!.flush();
      },
    );
  };

  return { liveAim, aimDrag, onReticleDown };
}
