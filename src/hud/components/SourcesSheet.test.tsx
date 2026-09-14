import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { SourcesSheet } from "./SourcesSheet";

// jsdom has no matchMedia; Motion asks for it.
vi.stubGlobal("matchMedia", () => ({ matches: false, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {} }));

const base: Parameters<typeof SourcesSheet>[0] = {
  targets: [{ id: "d1", label: "Built-in (2560x1600, Primary)" }, { id: "d2", label: "DELL U2720Q (3840x2160)" }],
  displayId: "d1", onTarget: vi.fn(),
  cameras: [{ id: "c1", label: "FaceTime HD" }, { id: "c2", label: "Link 2C" }], camId: "c1", onCam: vi.fn(),
  mics: [{ id: "m1", label: "Built-in" }, { id: "m2", label: "Yeti" }], micId: "m1", onMic: vi.fn(),
  menu: null, onMenu: vi.fn(), sheet: false, onSheet: vi.fn(),
};

let host: HTMLDivElement; let root: Root;
beforeEach(() => { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); });
afterEach(() => { act(() => root.unmount()); host.remove(); vi.clearAllMocks(); });
const render = (p: Partial<typeof base> = {}) => act(() => root.render(<SourcesSheet {...base} {...p} />));
// `.dd-label` is the row's own value line (a menu ITEM is `.dd-item-label`), so this is exactly
// what the three rows are showing as selected.
const labels = () => [...host.querySelectorAll(".dd-label")].map((e) => e.textContent);

describe("SourcesSheet", () => {
  it("is the three sources as rows, display first, each showing what is selected", () => {
    render();
    expect(host.querySelectorAll(".src-body .dd-row")).toHaveLength(3);
    expect(labels()).toEqual(["Built-in", "FaceTime HD", "Built-in"]);
  });

  it("the display row flips the sheet to the target list instead of opening a menu", () => {
    render();
    act(() => host.querySelector<HTMLButtonElement>(".src-body > .dd-row")!.click());
    expect(base.onSheet).toHaveBeenCalledWith(true);
    expect(base.onMenu).not.toHaveBeenCalled();
  });

  it("flipped, it is the same target list the idle card uses, and picking applies and returns", () => {
    render({ sheet: true });
    expect(host.querySelector(".sheet-title")?.textContent).toBe("What to record");
    const rows = host.querySelectorAll<HTMLButtonElement>(".dd-item.tgt");
    expect(rows).toHaveLength(2);
    act(() => rows[1].click());
    expect(base.onTarget).toHaveBeenCalledWith("d2");
    expect(base.onSheet).toHaveBeenCalledWith(false);
  });

  it("the camera and mic rows own their own menu ids, so one open menu never opens both", () => {
    render({ menu: "src-mic" });
    expect(host.querySelectorAll(".dd.open")).toHaveLength(1);
    const items = host.querySelectorAll<HTMLButtonElement>(".dd-menu .dd-item");
    expect([...items].map((i) => i.textContent)).toEqual(["Built-in", "Yeti"]);
    act(() => items[1].click());
    expect(base.onMic).toHaveBeenCalledWith("m2");
    expect(base.onCam).not.toHaveBeenCalled();
  });

  it("picking a camera reports the device, not the label", () => {
    render({ menu: "src-cam" });
    act(() => host.querySelectorAll<HTMLButtonElement>(".dd-menu .dd-item")[1].click());
    expect(base.onCam).toHaveBeenCalledWith("c2");
  });
});
