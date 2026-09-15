// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc, EditOp } from "../../../shared/edit";
import { useEditorKeymap } from "../../hooks/input/useEditorKeymap";
import { CutOverlay } from "./CutOverlay";

const DUR = 10_000;
const DOC = {
  cuts: [
    { id: "c0", start_ms: 1000, end_ms: 2000 },
    { id: "c1", start_ms: 4000, end_ms: 4500 },
  ],
  speed: [],
  zooms: [],
  effects: [],
  layout: [],
  camera_moves: [],
} as unknown as EditDoc;

let ops: EditOp[] = [];
let sel: string | null = null;
let root: Root, container: HTMLDivElement;

const all = (s: string) => [...container.querySelectorAll<HTMLElement>(s)];
const press = (el: Element | null) =>
  act(() => {
    el?.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
  });
const key = (k: string) =>
  act(() => {
    window.dispatchEvent(new KeyboardEvent("keydown", { key: k }));
  });

function Harness() {
  const [s, setS] = useState<string | null>(null);
  sel = s;
  const applyOp = async (op: EditOp) => {
    ops.push(op);
    return null;
  };
  useEditorKeymap({
    sel: s,
    doc: DOC,
    timeMs: 0,
    setSel: setS,
    setPlaying: () => {},
    applyOp,
    addZoom: async () => {},
    addSpotlight: async () => {},
    onOverlay: () => {},
    modalOpen: false,
    shortcutsOpen: false,
    escOwned: false,
  });
  return <CutOverlay cuts={DOC.cuts} dur={DUR} sel={s} onSel={setS} />;
}

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops = [];
  sel = null;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  act(() => {
    root.render(<Harness />);
  });
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("CutOverlay", () => {
  it("draws one hatched span per cut, at that cut's own share of the clip", () => {
    const spans = all(".e-cut");
    expect(spans).toHaveLength(2);
    expect([spans[0].style.left, spans[0].style.width]).toEqual(["10%", "10%"]);
    expect([spans[1].style.left, spans[1].style.width]).toEqual(["40%", "5%"]);
  });

  it("a click selects that cut, which is what opens its inspector", () => {
    press(all(".e-cut")[1]);
    expect(sel).toBe("c1");
    expect(all(".e-cut.sel")).toHaveLength(1);
    expect(ops).toEqual([]);
  });

  it("Delete on a selected cut removes it through the shared delete-selection path", () => {
    press(all(".e-cut")[0]);
    key("Delete");
    expect(ops).toEqual([{ op: "remove_cut", id: "c0" }]);
    expect(sel).toBeNull();
  });

  it("Delete with nothing selected removes nothing", () => {
    key("Delete");
    expect(ops).toEqual([]);
  });
});
