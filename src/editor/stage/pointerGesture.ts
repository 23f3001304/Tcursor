export interface GestureTarget {
  addEventListener(type: string, cb: (e: PointerEvent) => void): void;
  removeEventListener(type: string, cb: (e: PointerEvent) => void): void;
}

export function attachPointerGesture(
  onMove: (e: PointerEvent) => void,
  onEnd: () => void,
  target: GestureTarget = window,
): () => void {
  const detach = () => {
    target.removeEventListener("pointermove", onMove);
    target.removeEventListener("pointerup", end);
    target.removeEventListener("pointercancel", end);
  };
  const end = () => {
    detach();
    onEnd();
  };
  target.addEventListener("pointermove", onMove);
  target.addEventListener("pointerup", end);
  target.addEventListener("pointercancel", end);
  return detach;
}
