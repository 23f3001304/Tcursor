// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import type { InterfaceSettings, Settings } from "../../../hud/settings/settings";
import { applyProjectUi, mirrorUiToApp } from "./applyUi";

const ui = (over: Partial<InterfaceSettings> = {}): InterfaceSettings => ({
  theme: "dark",
  accent: [10, 20, 30],
  animated_brand: true,
  interface_effects: true,
  ai_choreography: false,
  ...over,
});

describe("applyProjectUi", () => {
  it("stamps the theme and the accent on the document root", () => {
    applyProjectUi(ui());
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("rgb(10, 20, 30)");
    applyProjectUi(ui({ theme: "light", accent: [1, 2, 3] }));
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("rgb(1, 2, 3)");
  });
});

describe("mirrorUiToApp", () => {
  it("writes only theme and accent into the app config, keeping every other field", async () => {
    const app = {
      ui: {
        theme: "light",
        accent: [0, 0, 0],
        animated_brand: false,
        interface_effects: false,
        ai_choreography: true,
      },
      cursor: { size: 2 },
    } as unknown as Settings;
    let written: Settings | undefined;
    const setSettings = vi.fn((s: Settings) => {
      written = s;
      return Promise.resolve();
    });
    await mirrorUiToApp(ui(), { getSettings: () => Promise.resolve(app), setSettings });
    expect(setSettings).toHaveBeenCalledTimes(1);
    expect(written?.ui).toEqual({
      theme: "dark",
      accent: [10, 20, 30],
      animated_brand: false,
      interface_effects: false,
      ai_choreography: true,
    });
    expect((written as unknown as { cursor: { size: number } }).cursor.size).toBe(2);
  });

  it("swallows an IPC failure so the dialog never throws", async () => {
    await expect(
      mirrorUiToApp(ui(), {
        getSettings: () => Promise.reject(new Error("no ipc")),
        setSettings: () => Promise.resolve(),
      }),
    ).resolves.toBeUndefined();
  });
});
