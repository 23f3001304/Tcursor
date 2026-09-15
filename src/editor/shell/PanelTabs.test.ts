// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { PANEL_TABS, type Tab } from "./PanelTabs";

describe("the panel tab strip", () => {
  it("has a Hotkeys tab and a separate Captions tab", () => {
    const ids = PANEL_TABS.map((t) => t.id);
    expect(ids).toContain("hotkeys");
    expect(ids).toContain("captions");
  });

  it("labels the hotkey overlay honestly", () => {
    expect(PANEL_TABS.find((t) => t.id === "hotkeys")?.label).toBe("Hotkeys");
    expect(PANEL_TABS.find((t) => t.id === "captions")?.label).toBe("Captions");
  });

  it("has no duplicate ids", () => {
    const ids: Tab[] = PANEL_TABS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
