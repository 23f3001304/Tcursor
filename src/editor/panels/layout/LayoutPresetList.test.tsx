// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { DEFAULT_APPEARANCE } from "../../../hud/preferences/appearanceFields";

vi.mock("../../../shared/ipc", async () => (await import("./layoutsFixture")).ipcMock());

const { h, BOLD, byText, rowNames, docWith, show, mountPanel, unmountPanel } =
  await import("./layoutsFixture");

const typeName = (name: string) => {
  const input = h.container.querySelector<HTMLInputElement>(".e-lay-input")!;
  act(() => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    setter.call(input, name);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
};

beforeEach(mountPanel);
afterEach(unmountPanel);

describe("LayoutPresetList", () => {
  it("lists the built-in Default row first, then the saved looks", async () => {
    h.app = { ...h.app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    expect(rowNames()).toEqual(["Default", "Bold"]);
    expect(h.container.querySelectorAll(".e-lay-kebab")).toHaveLength(1);
  });

  it("Apply writes the preset's whole five-layout appearance into the project", async () => {
    h.app = { ...h.app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    const applies = Array.from(h.container.querySelectorAll<HTMLButtonElement>(".e-lay-txt")).filter(
      (b) => b.textContent === "Apply",
    );
    act(() => {
      applies[1].click();
    });
    expect(h.saved).toHaveLength(1);
    expect(h.saved[0].appearance).toEqual(BOLD);
    expect(h.written).toHaveLength(0);
  });

  it("refuses an empty name, and a duplicate, rather than saving a look", async () => {
    h.app = { ...h.app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    act(() => {
      byText<HTMLButtonElement>(".e-upload", "Save current look").click();
    });
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Save").click();
    });
    expect(h.container.querySelector(".e-lay-err")?.textContent).toBe("Give this look a name.");
    expect(h.written).toHaveLength(0);

    typeName("bold");
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Save").click();
    });
    expect(h.container.querySelector(".e-lay-err")?.textContent).toContain("already a look called");
    expect(h.written).toHaveLength(0);
  });

  it("saves the project's current look into the app config, keeping the other global fields", async () => {
    const doc = docWith("screen");
    doc.settings.appearance = BOLD;
    await show(doc);
    act(() => {
      byText<HTMLButtonElement>(".e-upload", "Save current look").click();
    });
    typeName("Quiet");
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Save").click();
    });
    expect(h.written).toHaveLength(1);
    expect(h.written[0].layout_presets).toEqual([{ id: "lp1", name: "Quiet", appearance: BOLD }]);
    expect(h.written[0].appearance).toEqual(DEFAULT_APPEARANCE);
    expect(rowNames()).toEqual(["Default", "Quiet"]);
  });

  it("deletes a saved look through its kebab, and never writes the project doing it", async () => {
    h.app = { ...h.app, layout_presets: [{ id: "lp1", name: "Bold", appearance: BOLD }] };
    await show(docWith("screen"));
    act(() => {
      h.container.querySelector<HTMLButtonElement>(".e-lay-kebab")!.click();
    });
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Delete").click();
    });
    expect(h.written[0].layout_presets).toEqual([]);
    expect(h.saved).toHaveLength(0);
  });

  it("makes the project's look the default for new recordings, in the app config only", async () => {
    const doc = docWith("screen");
    doc.settings.appearance = BOLD;
    await show(doc);
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Make this the default for new recordings").click();
    });
    expect(h.written).toHaveLength(1);
    expect(h.written[0].appearance).toEqual(BOLD);
    expect(h.saved).toHaveLength(0);
  });
});
