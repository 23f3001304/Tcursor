// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc } from "../../shared/edit";
import { shellProps } from "./shellFixture";
import type { SlotProps } from "./slotProps";
import { PropertiesSlot, selectedClip } from "./PropertiesSlot";

const DOC = {
  zooms: [
    {
      id: "z0",
      start_ms: 0,
      end_ms: 1000,
      target: "cursor",
      scale: 2,
      easing: "smooth",
      zoom_in_ms: 350,
      zoom_out_ms: 450,
      layer: 0,
    },
  ],
  effects: [],
  layout: [],
  camera_moves: [],
  cuts: [{ id: "c0", start_ms: 1000, end_ms: 2500 }],
  speed: [{ id: "s0", start_ms: 3000, end_ms: 5000, factor: 2 }],
  aspect: "source",
  trim: { in_ms: 0, out_ms: 0 },
  settings: { ai_model: "", cursor: {}, clickfx: {}, zoom: {}, appearance: {}, ui: {} },
} as unknown as EditDoc;

let root: Root, container: HTMLDivElement;
const q = (s: string) => container.querySelector<HTMLElement>(s);
const labels = () => [...container.querySelectorAll(".e-fl")].map((e) => e.textContent ?? "");

const show = (sel: string | null) =>
  act(() => {
    const p = { ...shellProps({ doc: DOC, sel, dur: 10_000 }), onTab: () => {} } as SlotProps;
    root.render(<PropertiesSlot p={p} />);
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("selectedClip (what the sidebar exists for)", () => {
  it("is null with no selection at all, which is the collapsed sidebar", () => {
    expect(selectedClip(DOC, null)).toBeNull();
    expect(selectedClip(DOC, "")).toBeNull();
  });

  it("is null for a stale id that matches nothing, so a deleted clip collapses it too", () => {
    expect(selectedClip(DOC, "z-gone")).toBeNull();
  });

  it("names the kind and hands back the clip itself for each lane", () => {
    expect(selectedClip(DOC, "z0")).toEqual({ kind: "zoom", zoom: DOC.zooms[0] });
    expect(selectedClip(DOC, "c0")).toEqual({ kind: "cut", cut: DOC.cuts[0] });
    expect(selectedClip(DOC, "s0")).toEqual({ kind: "speed", speed: DOC.speed[0] });
  });
});

describe("PropertiesSlot routing", () => {
  it("a selected cut opens CutInspector, naming how much time it removes", () => {
    show("c0");
    expect(q(".e-ihead h2")?.textContent).toBe("Cut");
    expect(q(".e-isec-val")?.textContent).toContain("1.5 s");
    expect(q(".e-ihicon.del")?.getAttribute("aria-label")).toBe("Remove cut");
  });

  it("a selected speed span opens SpeedInspector, on its own factor", () => {
    show("s0");
    expect(q(".e-ihead h2")?.textContent).toBe("Speed");
    expect(labels().find((t) => t.startsWith("Factor"))).toBe("Factor 2x");
    expect(q('[role="slider"]')?.getAttribute("aria-valuenow")).toBe("2");
  });

  it("renders nothing at all with nothing selected - there is no empty panel to fill", () => {
    show(null);
    expect(q(".e-insp")).toBeNull();
    expect(container.textContent).toBe("");
  });
});
