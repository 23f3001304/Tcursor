import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc } from "../../lib/edit";
import { shellProps } from "./shellFixture";
import type { SlotProps } from "./slotProps";
import { PropertiesSlot } from "./PropertiesSlot";

// Routing only: which inspector one selection opens. The inspectors' own controls are covered where
// they live; what can silently rot here is the `sel`-to-inspector chain, since every new region kind
// has to be added to the same ladder.
const DOC = {
  zooms: [], effects: [], layout: [], camera_moves: [],
  cuts: [{ id: "c0", start_ms: 1000, end_ms: 2500 }],
  speed: [{ id: "s0", start_ms: 3000, end_ms: 5000, factor: 2 }],
  aspect: "source", trim: { in_ms: 0, out_ms: 0 },
  settings: { ai_model: "", cursor: {}, clickfx: {}, zoom: {}, appearance: {}, ui: {} },
} as unknown as EditDoc;

let root: Root, container: HTMLDivElement;
const q = (s: string) => container.querySelector<HTMLElement>(s);
const labels = () => [...container.querySelectorAll(".e-fl")].map((e) => e.textContent ?? "");

const show = (sel: string | null) => act(() => {
  const p = { ...shellProps({ doc: DOC, sel, dur: 10_000 }), onTab: () => {} } as SlotProps;
  root.render(<PropertiesSlot p={p} />);
});

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("PropertiesSlot routing", () => {
  it("a selected cut opens CutInspector, naming how much time it removes", () => {
    show("c0");
    expect(q(".e-phead h2")?.textContent).toBe("Cut");
    expect(q(".e-lede")?.textContent).toContain("1.5 s");
    expect(q(".e-del")?.textContent).toContain("Remove cut");
  });

  it("a selected speed span opens SpeedInspector, on its own factor", () => {
    show("s0");
    expect(q(".e-phead h2")?.textContent).toBe("Speed");
    expect(labels().find((t) => t.startsWith("Factor"))).toBe("Factor 2x");
    expect(q('[role="slider"]')?.getAttribute("aria-valuenow")).toBe("2");
  });

  it("nothing selected stays the one typographic sentence", () => {
    show(null);
    expect(q(".e-insp-empty")?.textContent).toContain("cut or speed span");
  });
});
