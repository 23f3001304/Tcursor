import { useEffect, useState, useSyncExternalStore, type CSSProperties, type RefObject } from "react";

/** How the composited frame sits in the stage area. `fit` is the long-standing behaviour. */
export type ViewMode = "fit" | "fill" | "native";

export const VIEW_MODES: { id: ViewMode; label: string; title: string }[] = [
  { id: "fit", label: "Fit", title: "Fit the whole frame inside the stage" },
  { id: "fill", label: "Fill", title: "Fill the stage, cropping the frame's long edge" },
  { id: "native", label: "100%", title: "Source pixels, centred - never upscaled" },
];

// Session-only UI state, deliberately NOT in the doc: nothing here is exported or persisted, and
// reopening the editor starts back at Fit. It lives in this module rather than in one component
// because the control (`ViewPicker`, in the transport row) and the thing it sizes (`Stage`) are
// siblings whose only shared parent is `ClassicShell`; a module store keeps the pair working
// without a prop channel through the shell. Lift it to `Editor` state the day something else
// needs to read it.
let mode: ViewMode = "fit";
const subs = new Set<() => void>();
const subscribe = (fn: () => void) => { subs.add(fn); return () => { subs.delete(fn); }; };

export function setViewMode(next: ViewMode): void {
  if (next === mode) return;
  mode = next;
  for (const fn of subs) fn();
}

export function useViewMode(): ViewMode {
  return useSyncExternalStore(subscribe, () => mode);
}

/** The stage box (which is also the canvas' displayed box, and the basis every on-stage overlay
 *  positions against) for one view mode, given the frame - the stage area's content box - in CSS
 *  px. `fit` keeps the aspect-ratio sizer the CSS already applies, so it needs no measurement;
 *  `fill` returns the COVER box, larger than the frame on one axis and cropped by the wrapper's
 *  own `overflow: hidden`; `native` returns the canvas' own pixel size, clamped down to the fit
 *  box so it is never upscaled. Pure - the geometry is pinned by a test, not by dragging a window. */
export function stageFrameStyle(view: ViewMode, canvasW: number, canvasH: number, frameW: number, frameH: number): CSSProperties {
  const ar = canvasW / canvasH;
  if (view === "fit" || frameW <= 0 || frameH <= 0) return { aspectRatio: `${canvasW} / ${canvasH}` };
  const w = view === "fill" ? Math.max(frameW, frameH * ar) : Math.min(canvasW, frameW, frameH * ar);
  return { width: Math.round(w), height: Math.round(w / ar), maxWidth: "none", maxHeight: "none" };
}

/** The element's live content-box size, `[0, 0]` until it is measured (and wherever there is no
 *  `ResizeObserver` at all, e.g. jsdom) - which `stageFrameStyle` reads as "stay on the Fit path". */
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
