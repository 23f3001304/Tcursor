import { describe, it, expect } from "vitest";
import { arrangeSeekMs, nextArrangeMode, NO_ARRANGE, type ArrangeMode } from "./arrangeMode";
import type { LayoutSeg } from "../../shared/edit";

const on = (segId: string): ArrangeMode => ({ on: true, segId });
const off = (segId: string): ArrangeMode => ({ on: false, segId });

describe("nextArrangeMode", () => {
  it("selecting a layout segment IS the entry gesture", () => {
    expect(nextArrangeMode(NO_ARRANGE, { kind: "select", segId: "s1" })).toEqual(on("s1"));
  });

  it("selecting a DIFFERENT layout segment moves the mode to it", () => {
    expect(nextArrangeMode(on("s1"), { kind: "select", segId: "s2" })).toEqual(on("s2"));
  });

  it("re-selecting the segment it is ALREADY arranging returns the same object", () => {
    const prev = on("s1");
    expect(nextArrangeMode(prev, { kind: "select", segId: "s1" })).toBe(prev);
    expect(nextArrangeMode(NO_ARRANGE, { kind: "select", segId: null })).toBe(NO_ARRANGE);
  });

  it("deselecting (or selecting a zoom/effect instead) exits and forgets the segment", () => {
    expect(nextArrangeMode(on("s1"), { kind: "select", segId: null })).toEqual(NO_ARRANGE);
    expect(nextArrangeMode(off("s1"), { kind: "select", segId: null })).toEqual(NO_ARRANGE);
  });

  it("Escape exits but REMEMBERS the segment, so the inspector button can re-enter", () => {
    const escaped = nextArrangeMode(on("s1"), { kind: "escape" });
    expect(escaped).toEqual(off("s1"));
    expect(nextArrangeMode(escaped, { kind: "arrange" })).toEqual(on("s1"));
  });

  it("Escape with the mode already off changes nothing (no phantom re-render)", () => {
    const prev = off("s1");
    expect(nextArrangeMode(prev, { kind: "escape" })).toBe(prev);
    expect(nextArrangeMode(NO_ARRANGE, { kind: "escape" })).toBe(NO_ARRANGE);
  });

  it("the arrange button is inert with nothing remembered", () => {
    expect(nextArrangeMode(NO_ARRANGE, { kind: "arrange" })).toBe(NO_ARRANGE);
  });

  it("re-selecting the SAME segment after an Escape re-enters", () => {
    expect(nextArrangeMode(off("s1"), { kind: "select", segId: "s1" })).toEqual(on("s1"));
  });

  it("the segment going away (deleted / doc reloaded without it) exits outright", () => {
    expect(nextArrangeMode(on("s1"), { kind: "gone" })).toEqual(NO_ARRANGE);
    expect(nextArrangeMode(off("s1"), { kind: "gone" })).toEqual(NO_ARRANGE);
  });
});

describe("arrangeSeekMs", () => {
  const seg = (over: Partial<LayoutSeg> = {}): LayoutSeg => ({
    id: "s1",
    start_ms: 1000,
    end_ms: 3000,
    layout: "camera",
    transition_ms: 200,
    easing: "smooth",
    transition_out_ms: 0,
    easing_out: "smooth",
    ...over,
  });

  it("is null while the playhead is already inside the span", () => {
    expect(arrangeSeekMs(seg(), 1000)).toBeNull();
    expect(arrangeSeekMs(seg(), 2999)).toBeNull();
  });

  it("lands past the entry transition, on the SETTLED layout rather than mid-fade", () => {
    expect(arrangeSeekMs(seg(), 0)).toBe(1200);
    expect(arrangeSeekMs(seg(), 3000)).toBe(1200);
  });

  it("clamps into the span when the transition is longer than the segment", () => {
    expect(arrangeSeekMs(seg({ transition_ms: 5000 }), 0)).toBe(2999);
  });

  it("never lands before the start, even on a zero-length segment", () => {
    expect(arrangeSeekMs(seg({ start_ms: 500, end_ms: 500, transition_ms: 0 }), 0)).toBe(500);
  });
});
