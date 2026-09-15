// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { act } from "react";
import { presetPatch } from "../motion/presets";
import {
  active,
  layoutAt,
  motionActive,
  motionSegs,
  ops,
  q,
  qa,
  segs,
  useInspectorDom,
  zoomAt,
} from "./inspectorFixture";

useInspectorDom();

describe("Zoom's Framing hero and grouped Timing row", () => {
  it("sets the scale as the section's hero value", () => {
    zoomAt({ scale: 2.8 });
    expect(q(".e-ihero-v")?.textContent).toBe("2.8x");
    expect(q(".e-ihero-l")?.textContent).toBe("Scale");
  });

  it("groups start, end, in and out as one row of four value fields", () => {
    zoomAt();
    const cells = qa('[aria-label="Timing"] .e-ival');
    expect(cells.map((c) => c.querySelector(".e-ival-l")?.textContent)).toEqual([
      "Start",
      "End",
      "In",
      "Out",
    ]);
    expect(
      cells
        .map((c) => c.querySelector(".e-ival-v")?.textContent)
        .slice(0, 1)
        .concat(cells.slice(2).map((c) => c.querySelector(".e-ival-v")?.textContent)),
    ).toEqual(["1.00s", "0.35s", "0.45s"]);
  });

  it("steps one cell by 0.05s without touching the other three", () => {
    zoomAt();
    act(() => {
      q<HTMLButtonElement>('[aria-label="More In"]')?.click();
    });
    expect(ops).toEqual([{ op: "update_zoom", id: "z1", zoom_in_ms: 400 }]);
  });

  it("clamps a cell at its own floor rather than going negative", () => {
    zoomAt({ zoom_in_ms: 0 });
    expect(q<HTMLButtonElement>('[aria-label="Less In"]')?.disabled).toBe(true);
  });
});

describe("the Motion section (M3): a preset row over the graph", () => {
  it("Zoom starts on Soft (the bare word smooth IS Soft) and a preset writes both ramps' curves", () => {
    zoomAt();
    expect(motionActive()).toEqual(["Soft"]);
    act(() => {
      motionSegs()[0].click();
    });
    const snappy = presetPatch("snappy");
    expect(ops).toEqual([
      { op: "update_zoom", id: "z1", easing: snappy.easing, easing_out: snappy.easing_out },
    ]);
  });

  it("says Custom on the heading, and lights no segment, when the curve pair matches no preset", () => {
    zoomAt({ easing: "cubic(0.100,0.200,0.300,0.400)" });
    expect(motionActive()).toEqual(["Custom"]);
    expect(qa(".e-isec-val")[1]?.textContent).toBe("Custom");
  });

  it("draws the curve's keys as draggable dots, even for a curve that has never been in keys form", () => {
    zoomAt();
    expect(qa('[aria-label="Motion preset"]').length).toBe(1);
    expect(qa(".e-mg-key").length).toBeGreaterThanOrEqual(2);
  });

  it("Layout's preset row draws a mini-canvas per preset and switches layout plus arrangement", () => {
    layoutAt();
    expect(active("Start from")).toEqual(["Camera"]);
    expect(qa('[aria-label="Start from"] .e-preseg-thumb').length).toBe(4);
    act(() => {
      segs("Start from")[1].click();
    });
    expect(ops[0]).toEqual({ op: "update_layout_seg", id: "l1", layout: "presenter" });
  });
});
