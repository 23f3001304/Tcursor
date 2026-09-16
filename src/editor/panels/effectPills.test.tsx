// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { TextKind } from "../../shared/edit";
import { EffectPills, PILLS, type PillActions } from "./EffectPills";

const actions = (): PillActions => ({
  onAddZoom: vi.fn(),
  onAddSpotlight: vi.fn(),
  onAddMask: vi.fn(),
  onAddLayout: vi.fn(),
  onAddCameraMove: vi.fn(),
  onAddText: vi.fn(),
});

describe("PILLS", () => {
  it("offers the three mask kinds beside the spotlight", () => {
    const names = PILLS.map((p) => p.name);
    for (const name of ["Blur Mask", "Pixelate Mask", "Highlight Mask"]) {
      expect(names).toContain(name);
    }
  });

  it("offers the four text kinds too", () => {
    const names = PILLS.map((p) => p.name);
    for (const name of ["Title", "Lower Third", "Big Stat", "Callout"]) {
      expect(names).toContain(name);
    }
  });

  it("still offers the four things it offered before", () => {
    const names = PILLS.map((p) => p.name);
    for (const name of ["Layout Segment", "Zoom Region", "Spotlight Highlight", "Camera Move"]) {
      expect(names).toContain(name);
    }
  });

  it("runs onAddMask with the kind the row names", () => {
    for (const kind of ["blur", "pixelate", "highlight"] as const) {
      const a = actions();
      const pill = PILLS.find((p) => p.type === kind);
      expect(pill, `no pill for ${kind}`).toBeTruthy();
      pill!.run(a);
      expect(a.onAddMask).toHaveBeenCalledWith(kind);
    }
  });

  it("runs onAddText with the kind the row names, off a text: drag type", () => {
    for (const kind of ["title", "lower_third", "stat", "callout"] as const) {
      const a = actions();
      const pill = PILLS.find((p) => p.type === `text:${kind}`);
      expect(pill, `no pill for ${kind}`).toBeTruthy();
      pill!.run(a);
      expect(a.onAddText).toHaveBeenCalledWith(kind);
    }
  });

  it("keeps every drag type distinct and gives the masks the fx accent", () => {
    expect(new Set(PILLS.map((p) => p.type)).size).toBe(PILLS.length);
    for (const kind of ["blur", "pixelate", "highlight"] as const) {
      expect(PILLS.find((p) => p.type === kind)!.cls).toBe("e-fxblk");
    }
    for (const kind of ["title", "lower_third", "stat", "callout"] as const) {
      expect(PILLS.find((p) => p.type === `text:${kind}`)!.cls).toBe("e-textblk");
    }
  });

  it("still routes the four original rows to their own actions", () => {
    const a = actions();
    for (const t of ["layout", "zoom", "spotlight", "cammove"]) PILLS.find((p) => p.type === t)!.run(a);
    expect(a.onAddLayout).toHaveBeenCalled();
    expect(a.onAddZoom).toHaveBeenCalled();
    expect(a.onAddSpotlight).toHaveBeenCalled();
    expect(a.onAddCameraMove).toHaveBeenCalled();
  });
});

let root: Root, container: HTMLDivElement;
let added: TextKind[];

const pills = () => [...container.querySelectorAll<HTMLElement>(".e-libpill")];
const named = (name: string) => pills().find((p) => p.querySelector(".e-libname")?.textContent === name);

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  added = [];
  act(() => {
    root.render(
      <EffectPills
        onAddZoom={() => {}}
        onAddSpotlight={() => {}}
        onAddMask={() => {}}
        onAddLayout={() => {}}
        onAddCameraMove={() => {}}
        onAddText={(k) => added.push(k)}
      />,
    );
  });
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

describe("EffectPills", () => {
  it("offers the four text kinds and seeds the one that was clicked", () => {
    for (const name of ["Title", "Lower Third", "Big Stat", "Callout"]) {
      expect(named(name), name).toBeTruthy();
    }
    act(() => {
      named("Lower Third")!.click();
    });
    expect(added).toEqual(["lower_third"]);
  });

  it("tags each text pill's drag payload with its own kind", () => {
    const setData = vi.fn();
    const ev = new Event("dragstart", { bubbles: true });
    Object.defineProperty(ev, "dataTransfer", { value: { setData, effectAllowed: "" } });
    act(() => {
      named("Big Stat")!.dispatchEvent(ev);
    });
    expect(setData).toHaveBeenCalledWith("text/plain", "text:stat");
  });
});
