import { describe, expect, it } from "vitest";
import { fmt, fmtPrecise, rulerTicks } from "./time";

describe("fmt", () => {
  it("formats whole seconds as M:SS", () => {
    expect(fmt(0)).toBe("0:00");
    expect(fmt(75_000)).toBe("1:15");
  });

  it("rounds to the nearest second rather than truncating", () => {
    expect(fmt(59_600)).toBe("1:00");
  });

  it("clamps negative values to zero", () => {
    expect(fmt(-500)).toBe("0:00");
  });
});

describe("fmtPrecise", () => {
  it("formats to one decisecond", () => {
    expect(fmtPrecise(12_400)).toBe("0:12.4");
    expect(fmtPrecise(0)).toBe("0:00.0");
  });

  it("carries a seconds value that rounds up to 60 into the next minute, never showing :60", () => {
    expect(fmtPrecise(59_940)).toBe("0:59.9");
    expect(fmtPrecise(59_960)).toBe("1:00.0");
    expect(fmtPrecise(119_970)).toBe("2:00.0");
  });
});

describe("rulerTicks", () => {
  it("returns a single 0:00 tick for a zero/negative duration", () => {
    expect(rulerTicks(0)).toEqual([{ at: 0, label: "0:00" }]);
  });

  it("never lands on the same rounding-carry bug for its own quantized tick times", () => {
    const ticks = rulerTicks(3_000);
    expect(ticks.some((t) => t.label.includes(":60"))).toBe(false);
  });
});
