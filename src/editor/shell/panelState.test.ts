import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { TAB_IDS } from "./panelTabs";
import { nextTab, readPanelTab, writePanelTab } from "./panelState";

beforeEach(() => { localStorage.clear(); });
afterEach(() => { vi.restoreAllMocks(); localStorage.clear(); });

describe("nextTab", () => {
  it("closes the panel when the tab already showing is pressed again", () => {
    expect(nextTab("camera", "camera")).toBeNull();
  });

  it("opens the pressed tab from any other tab, and from collapsed", () => {
    expect(nextTab("camera", "layouts")).toBe("layouts");
    expect(nextTab(null, "layouts")).toBe("layouts");
    expect(nextTab(null, "ai")).toBe("ai");
  });
});

describe("panel tab persistence", () => {
  it("round-trips an open tab", () => {
    writePanelTab("layouts");
    expect(readPanelTab(TAB_IDS)).toBe("layouts");
  });

  it("round-trips the collapsed state as the empty string", () => {
    writePanelTab(null);
    expect(localStorage.getItem("tcursor.editor.panel")).toBe("");
    expect(readPanelTab(TAB_IDS)).toBeNull();
  });

  it("opens on the AI tab when nothing is remembered or the value is from another build", () => {
    expect(readPanelTab(TAB_IDS)).toBe("ai");
    localStorage.setItem("tcursor.editor.panel", "hologram");
    expect(readPanelTab(TAB_IDS)).toBe("ai");
  });

  it("survives storage that throws outright, opening rather than collapsing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => { throw new Error("blocked"); });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("blocked"); });
    expect(readPanelTab(TAB_IDS)).toBe("ai");
    expect(() => writePanelTab("cursor")).not.toThrow();
  });
});
