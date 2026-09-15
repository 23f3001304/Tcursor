import { useEffect, useState, useSyncExternalStore, type CSSProperties, type RefObject } from "react";

export type ViewMode = "fit" | "fill" | "native";

export const VIEW_MODES: { id: ViewMode; label: string; title: string }[] = [
  { id: "fit", label: "Fit", title: "Fit the whole frame inside the stage" },
  { id: "fill", label: "Fill", title: "Fill the stage, cropping the frame's long edge" },
  { id: "native", label: "100%", title: "Source pixels, centred - never upscaled" },
];

let mode: ViewMode = "fit";
const subs = new Set<() => void>();
const subscribe = (fn: () => void) => {
  subs.add(fn);
  return () => {
    subs.delete(fn);
  };
};

export function setViewMode(next: ViewMode): void {
  if (next === mode) return;
  mode = next;
  for (const fn of subs) fn();
}

export function useViewMode(): ViewMode {
  return useSyncExternalStore(subscribe, () => mode);
}

export function stageFrameStyle(
  view: ViewMode,
  canvasW: number,
  canvasH: number,
  frameW: number,
  frameH: number,
): CSSProperties {
  const ar = canvasW / canvasH;
  if (view === "fit" || frameW <= 0 || frameH <= 0) return { aspectRatio: `${canvasW} / ${canvasH}` };
  const w = view === "fill" ? Math.max(frameW, frameH * ar) : Math.min(canvasW, frameW, frameH * ar);
  return { width: Math.round(w), height: Math.round(w / ar), maxWidth: "none", maxHeight: "none" };
}

export function useFrameSize(ref: RefObject<HTMLElement | null>): [number, number] {
  const [size, setSize] = useState<[number, number]>([0, 0]);
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      setSize((prev) => (prev[0] === width && prev[1] === height ? prev : [width, height]));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return size;
}
