import { useEffect, useMemo, useRef, useState } from "react";
import { debounce } from "../util/debounce";
import { parseKeys, type Keys } from "./keys";
import { buildGraph, GRAPH_H, GRAPH_W, type GraphInput } from "./graphModel";
import { clientToPlot, xToMs, yToProgress } from "./graphCoords";
import {
  addKeyAt,
  moveHandle,
  moveKey,
  nudgeKey,
  patchOf,
  removeKey,
  retimeIndex,
  retimeMs,
  toKeysInput,
  withCurve,
  type GraphPatch,
} from "./graphEdits";

export interface GraphSel {
  ramp: "in" | "out";
  key: number;
}

const COMMIT_DEBOUNCE_MS = 80;

const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, 1],
  ArrowDown: [0, -1],
};

export function useGraphDrag(
  input: GraphInput,
  onCommit: (patch: GraphPatch) => void,
  opts?: { readOnly?: boolean; retimeable?: boolean },
) {
  const readOnly = !!opts?.readOnly,
    retimeable = opts?.retimeable !== false;
  const sig = JSON.stringify(input);
  const base = useMemo(() => toKeysInput(JSON.parse(sig) as GraphInput), [sig]);
  const [draft, setDraft] = useState<GraphInput | null>(null);
  const [selected, setSelected] = useState<GraphSel | null>(null);
  useEffect(() => {
    setDraft(null);
  }, [sig]);
  const live = draft ?? base;
  const model = useMemo(() => buildGraph(live, GRAPH_W, GRAPH_H), [live]);
  const svg = useRef<SVGSVGElement>(null);

  const onCommitRef = useRef(onCommit);
  onCommitRef.current = onCommit;
  const deb = useRef<ReturnType<typeof debounce<[GraphPatch]>> | null>(null);
  if (!deb.current) deb.current = debounce((p: GraphPatch) => onCommitRef.current(p), COMMIT_DEBOUNCE_MS);
  useEffect(() => () => deb.current?.flush(), []);

  const curveOf = (which: "in" | "out") =>
    parseKeys((which === "in" ? live.rampIn.easing : live.rampOut?.easing) ?? "");
  const commitNow = (which: "in" | "out", k: Keys) => {
    setDraft(withCurve(live, which, k));
    deb.current!.cancel();
    onCommitRef.current(patchOf(which, k));
  };

  const drag = (which: "in" | "out", i: number, side: "in" | "out" | null) => (e: React.PointerEvent) => {
    const r = model.ramps.find((x) => x.which === which);
    const rect = svg.current?.getBoundingClientRect();
    const k0 = curveOf(which);
    if (readOnly || !r || !r.keys || !rect || !k0) return;
    e.preventDefault();
    e.stopPropagation();
    setSelected({ ramp: which, key: i });
    const w = Math.max(1e-6, r.x1 - r.x0);
    const isRetime = side === null && retimeable && i === retimeIndex(which, k0.keys.length);
    const move = (ev: PointerEvent) => {
      const [px, py] = clientToPlot(model, rect, ev.clientX, ev.clientY);
      const v = yToProgress(model, which, py),
        key = k0.keys[i];
      let k: Keys, dur: number | undefined;
      if (side) k = moveHandle(k0, i, side, (px - (r.x0 + key.t * w)) / w, v - key.v);
      else {
        k = moveKey(k0, i, (px - r.x0) / w, v);
        if (isRetime) dur = retimeMs(live, which, xToMs(model, live, px));
      }
      setDraft(withCurve(live, which, k, dur));
      deb.current!(patchOf(which, k, dur));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      for (const t of ["pointerup", "pointercancel"] as const) window.removeEventListener(t, up);
      deb.current!.flush();
    };
    window.addEventListener("pointermove", move);
    for (const t of ["pointerup", "pointercancel"] as const) window.addEventListener(t, up);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      setSelected(null);
      return;
    }
    const k = readOnly || !selected ? null : curveOf(selected.ramp);
    if (!k || !selected) return;
    const del = e.key === "Delete" || e.key === "Backspace";
    const next = del
      ? removeKey(k, selected.key)
      : ARROWS[e.key]
        ? nudgeKey(k, selected.key, ARROWS[e.key][0], ARROWS[e.key][1], e.shiftKey)
        : k;
    if (next === k) return;
    e.preventDefault();
    if (del) setSelected(null);
    commitNow(selected.ramp, next);
  };

  const onDoubleClick = (e: React.MouseEvent) => {
    const rect = svg.current?.getBoundingClientRect();
    if (readOnly || !rect) return;
    const [px] = clientToPlot(model, rect, e.clientX, e.clientY);
    const r = model.ramps.find((x) => x.keys && px >= x.x0 && px <= x.x1);
    const k = r ? curveOf(r.which) : null;
    if (!r || !k) return;
    const next = addKeyAt(k, (px - r.x0) / Math.max(1e-6, r.x1 - r.x0));
    if (next !== k) commitNow(r.which, next);
  };

  const handlers = {
    svg: {
      ref: svg,
      tabIndex: 0,
      onKeyDown,
      onDoubleClick,
      onPointerDown: (e: React.PointerEvent) => {
        if (e.target === svg.current) setSelected(null);
      },
    },
    key: (which: "in" | "out", i: number) => ({
      tabIndex: readOnly ? -1 : 0,
      role: "button",
      "aria-label": `${which === "in" ? "In" : "Out"} ramp key ${i + 1}`,
      onFocus: () => setSelected({ ramp: which, key: i }),
      onPointerDown: drag(which, i, null),
    }),
    handle: (which: "in" | "out", i: number, side: "in" | "out") => ({
      "aria-hidden": true,
      onPointerDown: drag(which, i, side),
    }),
  };

  return { model, selected, handlers, live };
}
