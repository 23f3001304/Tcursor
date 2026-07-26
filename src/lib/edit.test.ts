import { describe, expect, it } from "vitest";
import { resolveTrim } from "./edit";

describe("resolveTrim", () => {
  it("treats out_ms === 0 as no trim yet - the whole clip", () => {
    expect(resolveTrim({ in_ms: 0, out_ms: 0 }, 12_345)).toEqual({ inMs: 0, outMs: 12_345 });
  });

  it("returns the trim range unchanged when it fits inside the clip", () => {
    expect(resolveTrim({ in_ms: 2_000, out_ms: 8_000 }, 10_000)).toEqual({ inMs: 2_000, outMs: 8_000 });
  });

  it("clamps out_ms beyond the real duration down to the clip length", () => {
    expect(resolveTrim({ in_ms: 2_000, out_ms: 999_999 }, 10_000)).toEqual({ inMs: 2_000, outMs: 10_000 });
  });

  it("clamps in_ms to the resolved out_ms instead of going negative-length", () => {
    expect(resolveTrim({ in_ms: 9_000, out_ms: 5_000 }, 10_000)).toEqual({ inMs: 5_000, outMs: 5_000 });
  });
});
