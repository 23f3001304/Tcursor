// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { act } from "react";
import { EffectInspector } from "./EffectInspector";
import { CameraMoveInspector } from "./CameraMoveInspector";
import { CutInspector } from "./CutInspector";
import { SpeedInspector } from "./SpeedInspector";
import {
  apply,
  CUT,
  del,
  FX,
  layoutAt,
  MOVE,
  noop,
  ops,
  q,
  range,
  sections,
  SETTINGS,
  show,
  SPEED,
  useInspectorDom,
  zoomAt,
} from "./inspectorFixture";

useInspectorDom();

describe("one inspector shape (sections in DOM order)", () => {
  it("Zoom reads framing, timing, motion, webcam", () => {
    zoomAt();
    expect(sections()).toEqual(["Framing", "Timing", "Motion", "Webcam during zoom"]);
    expect(range()).toBe("1.00s to 3.60s");
    expect(del()?.getAttribute("aria-label")).toBe("Delete zoom");
  });

  it("Spotlight reads timing, look, fades", () => {
    show(
      <EffectInspector
        effect={FX}
        dur={10_000}
        settings={SETTINGS}
        onApply={apply}
        onDimCamera={noop}
        onClose={noop}
      />,
    );
    expect(sections()).toEqual(["Timing", "Look", "Fades"]);
    expect(range()).toBe("0.00s to 2.00s");
  });

  it("Layout reads timing, composition, motion", () => {
    layoutAt();
    expect(sections()).toEqual(["Timing", "Composition", "Motion"]);
  });

  it("Camera Move reads timing, placement, shape, motion", () => {
    show(<CameraMoveInspector move={MOVE} moves={[]} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Placement", "Shape", "Motion"]);
    expect(range()).toBe("Keyframe at 1.20s");
  });

  it("Cut reads timing only, and says how much it removes", () => {
    show(<CutInspector cut={CUT} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing"]);
    expect(q(".e-isec-val")?.textContent).toBe("removes 1.5 s");
  });

  it("Speed reads timing, rate", () => {
    show(<SpeedInspector speed={SPEED} dur={10_000} onApply={apply} onClose={noop} />);
    expect(sections()).toEqual(["Timing", "Rate"]);
  });

  it("puts Delete in the header of every inspector, and a section last", () => {
    zoomAt();
    const panel = q(".e-insp");
    expect(panel?.firstElementChild?.className).toContain("e-ihead");
    expect(panel?.lastElementChild?.className).toContain("e-isec");
    act(() => {
      del()?.click();
    });
    expect(ops).toEqual([{ op: "remove_zoom", id: "z1" }]);
  });
});
