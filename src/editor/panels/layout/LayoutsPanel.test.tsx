// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { DEFAULT_APPEARANCE } from "../../../hud/preferences/appearanceFields";

vi.mock("../../../shared/ipc", async () => (await import("./layoutsFixture")).ipcMock());

const { h, byText, docWith, show, mountPanel, unmountPanel } = await import("./layoutsFixture");

const checked = () => h.container.querySelector('.e-picker-btn[aria-label="Layout"]')?.textContent;
const pickLayout = (label: string) => {
  act(() => {
    h.container.querySelector<HTMLButtonElement>('.e-picker-btn[aria-label="Layout"]')!.click();
  });
  act(() => {
    Array.from(document.querySelectorAll<HTMLButtonElement>('[role="option"]'))
      .find((e) => e.textContent?.trim() === label)!
      .click();
  });
};

beforeEach(mountPanel);
afterEach(unmountPanel);

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
    pickLayout("Camera only");
    expect(checked()).toBe("Camera only");
    expect(h.saved).toHaveLength(0);
  });

  it("a knob change patches that layout's appearance and leaves the other four alone", async () => {
    await show(docWith("presenter"));
    act(() => {
      byText<HTMLButtonElement>(".e-segment", "Rect").click();
    });
    expect(h.saved).toHaveLength(1);
    expect(h.saved[0].appearance.presenter.cam_shape).toBe("rect");
    expect(h.saved[0].appearance.screen).toEqual(DEFAULT_APPEARANCE.screen);
    expect(h.saved[0].ai_model).toBe("");
  });

  it("resets one layout back to its default without touching the others", async () => {
    const doc = docWith("camera_only");
    doc.settings.appearance = {
      ...DEFAULT_APPEARANCE,
      camera_only: { ...DEFAULT_APPEARANCE.camera_only, pad: 0.07 },
    };
    await show(doc);
    act(() => {
      byText<HTMLButtonElement>(".e-lay-txt", "Reset this layout").click();
    });
    expect(h.saved[0].appearance.camera_only).toEqual(DEFAULT_APPEARANCE.camera_only);
    expect(h.saved[0].appearance.presenter).toBe(doc.settings.appearance.presenter);
  });
});
