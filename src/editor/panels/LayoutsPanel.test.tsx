import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc, LayoutSeg } from "../../lib/edit";
import type { Settings } from "../../hud/settings/settings";
import { DEFAULT_APPEARANCE } from "../../hud/preferences/appearanceFields";

// The panel reads and writes the APP config over IPC (saved looks are global, not per project);
// that seam does not exist in jsdom, so it is stubbed. Everything else - the layout picker, the
// knobs, the preset rows, the name field - is the real component.
const BOLD = { ...DEFAULT_APPEARANCE, presenter: { ...DEFAULT_APPEARANCE.presenter, pad: 0.07 } };
let app: Settings;
let written: Settings[] = [];
vi.mock("../../lib/ipc", () => ({
  getSettings: () => Promise.resolve(app),
  setSettings: (s: Settings) => { written.push(s); return Promise.resolve(); },
}));

const { LayoutsPanel } = await import("./LayoutsPanel");

const seg = (layout: string): LayoutSeg =>
  ({ id: "l1", start_ms: 0, end_ms: 2000, layout, transition_ms: 0, easing: "smooth", transition_out_ms: 0, easing_out: "smooth" });

const docWith = (layout: string): EditDoc =>
  ({ layout: [seg(layout)], settings: { ai_model: "", appearance: DEFAULT_APPEARANCE, layout_presets: [] } }) as unknown as EditDoc;

let root: Root, container: HTMLDivElement;
let saved: EditDoc["settings"][] = [];

const byText = <T extends HTMLElement>(sel: string, text: string) =>
  Array.from(container.querySelectorAll<T>(sel)).find((e) => e.textContent?.trim() === text)!;
const rowNames = () => Array.from(container.querySelectorAll(".e-lay-name")).map((e) => e.textContent);
const checked = () => container.querySelector('[role="radiogroup"][aria-label="Layout"] [aria-checked="true"]')?.textContent;

const show = async (doc: EditDoc, timeMs = 500) => {
  await act(async () => {
    root.render(<LayoutsPanel doc={doc} timeMsRef={{ current: timeMs }}
      onSaveSettings={(s) => saved.push(s)} onClose={() => {}} />);
  });
};

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  saved = []; written = [];
  app = { ai_model: "", appearance: DEFAULT_APPEARANCE, layout_presets: [] } as unknown as Settings;
  localStorage.clear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); localStorage.clear(); });

describe("LayoutsPanel", () => {
  it("opens on the layout of the segment under the playhead", async () => {
    await show(docWith("presenter"));
    expect(checked()).toBe("Presenter");
  });

  it("opens on Screen when the playhead is in no layout segment", async () => {
    await show(docWith("presenter"), 5000);
    expect(checked()).toBe("Screen");
  });

  it("picking a different layout writes nothing - it only changes what is being edited", async () => {
    await show(docWith("presenter"));
    act(() => { byText<HTMLButtonElement>(".e-segment", "Camera only").click(); });
    expect(checked()).toBe("Camera only");
    expect(saved).toHaveLength(0);
  });

  it("a knob change patches that layout's appearance and leaves the other four alone", async () => {
    await show(docWith("presenter"));
    act(() => { byText<HTMLButtonElement>(".e-segment", "Rect").click(); });
    expect(saved).toHaveLength(1);
    expect(saved[0].appearance.presenter.cam_shape).toBe("rect");
    expect(saved[0].appearance.screen).toEqual(DEFAULT_APPEARANCE.screen);
    expect(saved[0].ai_model).toBe(""); // the rest of the doc settings survive
  });

  it("resets one layout back to its default without touching the others", async () => {
    const doc = docWith("camera_only");
    doc.settings.appearance = { ...DEFAULT_APPEARANCE, camera_only: { ...DEFAULT_APPEARANCE.camera_only, pad: 0.07 } };
    await show(doc);
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Reset this layout").click(); });
    expect(saved[0].appearance.camera_only).toEqual(DEFAULT_APPEARANCE.camera_only);
    expect(saved[0].appearance.presenter).toBe(doc.settings.appearance.presenter);
  });

  it("lists the built-in Default row first, then the saved looks", async () => {
    app = { ...app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    expect(rowNames()).toEqual(["Default", "Bold"]);
    // The built-in row has no kebab, so it cannot be renamed or deleted.
    expect(container.querySelectorAll(".e-lay-kebab")).toHaveLength(1);
  });

  it("Apply writes the preset's whole five-layout appearance into the project", async () => {
    app = { ...app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    const applies = Array.from(container.querySelectorAll<HTMLButtonElement>(".e-lay-txt"))
      .filter((b) => b.textContent === "Apply");
    act(() => { applies[1].click(); }); // the saved look, not the built-in row
    expect(saved).toHaveLength(1);
    expect(saved[0].appearance).toEqual(BOLD);
    expect(written).toHaveLength(0); // applying a look never rewrites the app config
  });

  it("refuses an empty name, and a duplicate, rather than saving a look", async () => {
    app = { ...app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    act(() => { byText<HTMLButtonElement>(".e-upload", "Save current look").click(); });
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Save").click(); });
    expect(container.querySelector(".e-lay-err")?.textContent).toBe("Give this look a name.");
    expect(written).toHaveLength(0);

    const input = container.querySelector<HTMLInputElement>(".e-lay-input")!;
    act(() => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
      setter.call(input, "bold");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Save").click(); });
    expect(container.querySelector(".e-lay-err")?.textContent).toContain("already a look called");
    expect(written).toHaveLength(0);
  });

  it("saves the project's current look into the app config, keeping the other global fields", async () => {
    const doc = docWith("screen");
    doc.settings.appearance = BOLD;
    await show(doc);
    act(() => { byText<HTMLButtonElement>(".e-upload", "Save current look").click(); });
    const input = container.querySelector<HTMLInputElement>(".e-lay-input")!;
    act(() => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
      setter.call(input, "Quiet");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Save").click(); });
    expect(written).toHaveLength(1);
    expect(written[0].layout_presets).toEqual([{ id: "lp1", name: "Quiet", appearance: BOLD }]);
    expect(written[0].appearance).toEqual(DEFAULT_APPEARANCE); // the global default is untouched
    expect(rowNames()).toEqual(["Default", "Quiet"]);
  });

  it("deletes a saved look through its kebab, and never writes the project doing it", async () => {
    app = { ...app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    act(() => { container.querySelector<HTMLButtonElement>(".e-lay-kebab")!.click(); });
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Delete").click(); });
    expect(written[0].layout_presets).toEqual([]);
    expect(saved).toHaveLength(0);
  });

  it("makes the project's look the default for new recordings, in the app config only", async () => {
    const doc = docWith("screen");
    doc.settings.appearance = BOLD;
    await show(doc);
    act(() => { byText<HTMLButtonElement>(".e-lay-txt", "Make this the default for new recordings").click(); });
    expect(written).toHaveLength(1);
    expect(written[0].appearance).toEqual(BOLD);
    expect(saved).toHaveLength(0);
  });
});
