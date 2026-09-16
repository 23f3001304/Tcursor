import { describe, it, expect } from "vitest";
import type { TextItem } from "../../../shared/edit";
import { textPreviewText, textTitle, useTextLaneRegions, TEXT_PILL_CHARS } from "./TextLane";

const item = (id: string, start_ms: number, end_ms: number, text: string): TextItem => ({
  id,
  start_ms,
  end_ms,
  kind: "title",
  text,
  sub: null,
  style: "clean",
  pos: "mid_center",
  offset: [0, 0],
  size: "l",
  anim_in: "fade",
  anim_out: "fade",
  in_ms: 420,
  out_ms: 420,
  easing: "smooth",
});

describe("TextLane", () => {
  it("labels a pill with the first characters of the line", () => {
    expect(textPreviewText("Ship it on Friday", TEXT_PILL_CHARS)).toBe("Ship it on Friday");
    const long = "A title that is far longer than the pill can ever show";
    expect(textPreviewText(long, TEXT_PILL_CHARS).length).toBeLessThanOrEqual(TEXT_PILL_CHARS);
    expect(textTitle({ ...item("t0", 0, 1, long), layer: 0 })).toBe(long);
  });

  it("gives an untitled item something to show rather than an empty pill", () => {
    expect(textPreviewText("   ", TEXT_PILL_CHARS)).toBe("Text");
    expect(textTitle({ ...item("t0", 0, 1, "   "), layer: 0 })).toBeUndefined();
  });

  it("stacks overlapping items onto separate rows", () => {
    const rows = useTextLaneRegions.pure([
      item("t0", 0, 2000, "one"),
      item("t1", 1000, 3000, "two"),
      item("t2", 4000, 5000, "three"),
    ]);
    expect(rows.map((r) => r.layer)).toEqual([0, 1, 0]);
  });
});
