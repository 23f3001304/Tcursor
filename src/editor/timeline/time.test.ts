import { describe, expect, it } from "vitest";
import { fmt, fmtPrecise, rulerTicks } from "./time";

describe("fmt", () => {
  it("formats whole seconds as M:SS", () => {
    expect(fmt(0)).toBe("0:00");
    expect(fmt(75_000)).toBe("1:15");
  });

  it("rounds to the nearest second rather than truncating", () => {
    expect(fmt(59_600)).toBe("1:00"); // 59.6s rounds up, not "0:60"
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

  // Fix round 1 (design/premium-pass D3 review): a live, un-quantized `ms` can land just under a
  // minute boundary and round UP to :60 at the display precision - the readout must carry that
  // into the next minute instead of ever showing a "60" in the seconds field.
  it("carries a seconds value that rounds up to 60 into the next minute, never showing :60", () => {
    expect(fmtPrecise(59_940)).toBe("0:59.9"); // rounds down - no carry needed
    expect(fmtPrecise(59_960)).toBe("1:00.0"); // rounds up to 60.0 - must carry
    expect(fmtPrecise(119_970)).toBe("2:00.0"); // same boundary one minute later
  });
});

describe("rulerTicks", () => {
  it("returns a single 0:00 tick for a zero/negative duration", () => {
    expect(rulerTicks(0)).toEqual([{ at: 0, label: "0:00" }]);
  });

  it("never lands on the same rounding-carry bug for its own quantized tick times", () => {
    // A short clip picks a sub-second (decimal-label) step; every tick's `ms` is an exact multiple
    // of that step, so none of them should ever land on a label containing ":60".
    const ticks = rulerTicks(3_000);
    expect(ticks.some((t) => t.label.includes(":60"))).toBe(false);
  });
});
