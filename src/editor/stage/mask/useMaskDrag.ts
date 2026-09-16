import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { EffectRegion } from "../../../shared/edit";
import type { PreviewLayout } from "../../../shared/ipc";
import { pastDragThreshold } from "../../util/dragThreshold";
import { fxFrameGeometry } from "../fx/fxGeometry";
import { maskDraws, type MaskPx } from "./maskPreview";

export type Rect = [number, number, number, number];
export type MaskHandle = "move" | "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se";

export const SNAP_TOL = 0.01;
const MIN = 0.01;

export function clampMaskRect(r: Rect): Rect | null {
  if (r.some((v) => !Number.isFinite(v))) return null;
  const w = Math.min(1, Math.max(MIN, r[2]));
  const h = Math.min(1, Math.max(MIN, r[3]));
  return [Math.min(1 - w, Math.max(0, r[0])), Math.min(1 - h, Math.max(0, r[1])), w, h];
}

export function resizeMaskRect(r: Rect, handle: MaskHandle, dx: number, dy: number): Rect {
  if (handle === "move") return [r[0] + dx, r[1] + dy, r[2], r[3]];
  let [x, y, w, h] = r;
  if (handle.includes("w")) {
    x += dx;
    w -= dx;
  }
  if (handle.includes("e")) w += dx;
  if (handle.includes("n")) {
    y += dy;
    h -= dy;
  }
  if (handle.includes("s")) h += dy;
  return [x, y, w, h];
}

const LINES = [0, 0.5, 1];

function snapAxis(lo: number, size: number, tol: number): [number, number | null] {
  const mid = lo + size / 2;
  for (const line of LINES) {
    if (Math.abs(lo - line) <= tol) return [line, line];
    if (Math.abs(lo + size - line) <= tol) return [line - size, line];
    if (Math.abs(mid - line) <= tol) return [line - size / 2, line];
  }
  return [lo, null];
}

export function snapMaskRect(r: Rect, tol: number) {
  const [x, guideX] = snapAxis(r[0], r[2], tol);
  const [y, guideY] = snapAxis(r[1], r[3], tol);
  return { rect: [x, y, r[2], r[3]] as Rect, guideX, guideY };
}

export function useMaskDrag({
  effect,
  layout,
  cam,
  canvasW,
  canvasH,
  tOut,
  defaultDim,
  dirtyRef,
  onCommit,
}: {
  effect: EffectRegion | null;
  layout: PreviewLayout | null;
  cam: { cx: number; cy: number; scale: number };
  canvasW: number;
  canvasH: number;
  tOut: number;
  defaultDim: number;
  dirtyRef: RefObject<boolean>;
  onCommit: (id: string, rect: Rect) => void;
}) {
  const [draft, setDraft] = useState<Rect | null>(null);
  const [guides, setGuides] = useState<{ x: number | null; y: number | null }>({
    x: null,
    y: null,
  });
  const from = useRef({
    x: 0,
    y: 0,
    rect: [0, 0, 0, 0] as Rect,
    sx: 1,
    sy: 1,
    handle: "move" as MaskHandle,
  });
  const live = useRef<Rect | null>(null);

  const onHandleDown = useCallback(
    (e: React.PointerEvent, handle: MaskHandle) => {
      if (!effect?.rect) return;
      e.stopPropagation();
      const g = fxFrameGeometry(canvasW, canvasH, layout, cam, 1);
      const a = g.mapCanvas(0, 0);
      const b = g.mapCanvas(1, 1);
      if (!a || !b) return;
      from.current = {
        x: e.clientX,
        y: e.clientY,
        rect: effect.rect,
        sx: b[0] - a[0] || 1,
        sy: b[1] - a[1] || 1,
        handle,
      };
      live.current = effect.rect;
      setDraft(effect.rect);
    },
    [effect, layout, cam, canvasW, canvasH],
  );

  const dragging = draft !== null;
  const id = effect?.id ?? null;
  useEffect(() => {
    if (!dragging || !id) return;
    const el = document.querySelector<HTMLElement>(".e-canvas");
    const scale = el ? el.getBoundingClientRect().width / Math.max(1, canvasW) : 1;
    const move = (e: PointerEvent) => {
      const f = from.current;
      const dx = (e.clientX - f.x) / Math.max(scale, 1e-6) / f.sx;
      const dy = (e.clientY - f.y) / Math.max(scale, 1e-6) / f.sy;
      const raw = clampMaskRect(resizeMaskRect(f.rect, f.handle, dx, dy));
      if (!raw) return;
      const s = snapMaskRect(raw, SNAP_TOL);
      const next = clampMaskRect(s.rect) ?? raw;
      live.current = next;
      setDraft(next);
      setGuides({ x: s.guideX, y: s.guideY });
      dirtyRef.current = true;
    };
    const up = (e: PointerEvent) => {
      const f = from.current;
      const moved = pastDragThreshold(e.clientX - f.x, e.clientY - f.y);
      const next = live.current;
      if (moved && next) onCommit(id, next);
      live.current = null;
      setDraft(null);
      setGuides({ x: null, y: null });
      dirtyRef.current = true;
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [dragging, id, canvasW, dirtyRef, onCommit]);

  const shown = effect && draft ? { ...effect, rect: draft } : effect;
  const box: MaskPx | null = shown
    ? (maskDraws([shown], layout, cam, canvasW, canvasH, tOut, defaultDim)[0] ?? null)
    : null;
  return { box, guideX: guides.x, guideY: guides.y, onHandleDown };
}
