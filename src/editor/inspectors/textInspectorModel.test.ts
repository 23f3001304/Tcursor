import { describe, it, expect } from "vitest";
import type { TextItem } from "../../shared/edit";
import { ANCHOR_GRID, slideIsPointless, textGraphInput } from "./textInspectorModel";

const item: TextItem = {
  id: "t0",
  start_ms: 1000,
  end_ms: 5000,
  kind: "title",
  text: "Hello",
  sub: null,
  style: "clean",
  pos: "mid_center",
  offset: [0, 0],
  size: "l",
  anim_in: "slide",
  anim_out: "fade",
  in_ms: 1200,
  out_ms: 800,
  easing: "smooth",
};

describe("textInspectorModel", () => {
  it("lays the nine anchors out in reading order, three by three", () => {
    expect(ANCHOR_GRID).toEqual([
      "top_left",
      "top_center",
      "top_right",
      "mid_left",
      "mid_center",
      "mid_right",
      "bottom_left",
      "bottom_center",
      "bottom_right",
    ]);
  });

  it("greys Slide out at the centre, where it has no direction", () => {
    expect(slideIsPointless("mid_center")).toBe(true);
    for (const a of ANCHOR_GRID.filter((x) => x !== "mid_center")) {
      expect(slideIsPointless(a)).toBe(false);
    }
  });

  it("feeds the motion graph the item's own span and ramps", () => {
    const g = textGraphInput(item);
    expect([g.startMs, g.endMs]).toEqual([1000, 5000]);
    expect(g.rampIn).toEqual({ easing: "smooth", durMs: 1200 });
    expect(g.rampOut).toEqual({ easing: "smooth", durMs: 800 });
    expect(g.peak).toBe(1);
    expect(g.lane).toBe("text");
  });
});
