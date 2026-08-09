import { describe, it, expect } from "vitest";
import { flowSeconds, dotPulses, dotTint, WAVE_LAMBDA } from "./brandWave";

describe("flowSeconds", () => {
  it("idle never animates", () => {
    expect(flowSeconds("idle")).toBe(0);
    expect(flowSeconds("idle", 50)).toBe(0);
  });

  it("recording and directing flow at a fixed 2s/lambda regardless of pct", () => {
    expect(flowSeconds("recording")).toBe(2);
    expect(flowSeconds("directing", 80)).toBe(2);
  });

  it("exporting speeds up from 3s at 0% to 0.8s at 100%", () => {
    expect(flowSeconds("exporting", 0)).toBeCloseTo(3);
    expect(flowSeconds("exporting", 100)).toBeCloseTo(0.8);
    expect(flowSeconds("exporting", 50)).toBeCloseTo(1.9); // midpoint of 3 -> 0.8
  });

  it("exporting clamps pct outside 0..100", () => {
    expect(flowSeconds("exporting", -20)).toBeCloseTo(3);
    expect(flowSeconds("exporting", 250)).toBeCloseTo(0.8);
  });

  it("exporting defaults pct to 0 (slowest) when omitted", () => {
    expect(flowSeconds("exporting")).toBeCloseTo(3);
  });
});

describe("dotPulses", () => {
  it("only recording pulses", () => {
    expect(dotPulses("recording")).toBe(true);
    expect(dotPulses("idle")).toBe(false);
    expect(dotPulses("exporting")).toBe(false);
    expect(dotPulses("directing")).toBe(false);
  });
});

describe("dotTint", () => {
  it("only directing overrides the dot color", () => {
    expect(dotTint("directing")).toBe("var(--e-ai)");
    expect(dotTint("idle")).toBeNull();
    expect(dotTint("recording")).toBeNull();
    expect(dotTint("exporting")).toBeNull();
  });
});

describe("WAVE_LAMBDA", () => {
  it("is a positive period", () => {
    expect(WAVE_LAMBDA).toBeGreaterThan(0);
  });
});
