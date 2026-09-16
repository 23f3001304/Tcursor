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
  effects: [
    { id: "fx0", kind: "spotlight", start_ms: 0, end_ms: 2000, fade_in_ms: 250, fade_out_ms: 250, layer: 0 },
    {
      id: "mk0",
      kind: "blur",
      start_ms: 0,
      end_ms: 3000,
      fade_in_ms: 250,
      fade_out_ms: 250,
      layer: 0,
      rect: [0.35, 0.4, 0.3, 0.2],
    },
  ],
  layout: [],
  camera_moves: [],
  texts: [
    {
      id: "t0",
      start_ms: 2000,
      end_ms: 5000,
      kind: "lower_third",
      text: "Hemang",
      sub: "Owner",
      style: "bar",
      pos: "bottom_left",
      offset: [0, 0],
      size: "m",
      anim_in: "slide",
      anim_out: "fade",
      in_ms: 420,
      out_ms: 420,
      easing: "smooth",
    },
  ],
  clips: [],
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
    expect(selectedClip(DOC, "t0")).toEqual({ kind: "text", text: DOC.texts[0] });
  });

  it("splits the one effects list by kind, so a mask is not a spotlight", () => {
    expect(selectedClip(DOC, "fx0")).toEqual({ kind: "fx", effect: DOC.effects[0] });
    expect(selectedClip(DOC, "mk0")).toEqual({ kind: "mask", effect: DOC.effects[1] });
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

  it("a selected mask opens MaskInspector, titled and labelled by its kind", () => {
    show("mk0");
    expect(q(".e-ihead h2")?.textContent).toBe("Blur");
    expect(q(".e-ihicon.del")?.getAttribute("aria-label")).toBe("Delete blur");
  });

  it("a selected spotlight still opens the spotlight inspector, untouched", () => {
    show("fx0");
    expect(q(".e-ihead h2")?.textContent).toBe("Spotlight");
  });

  it("a selected text item opens TextInspector, headed by its kind", () => {
    show("t0");
    expect(q(".e-ihead h2")?.textContent).toBe("Lower third");
    expect(q(".e-ihicon.del")?.getAttribute("aria-label")).toBe("Delete lower third");
    expect((q(".e-itextarea") as HTMLTextAreaElement | null)?.value).toBe("Hemang");
    expect((q(".e-isubinput") as HTMLInputElement | null)?.value).toBe("Owner");
  });

  it("renders nothing at all with nothing selected - there is no empty panel to fill", () => {
    show(null);
    expect(q(".e-insp")).toBeNull();
    expect(container.textContent).toBe("");
  });
});
