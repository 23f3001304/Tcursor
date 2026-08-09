import { describe, expect, it } from "vitest";
import { timeBucket, fxCacheKey, isStaleFxResponse } from "./fxCacheKey";

describe("timeBucket", () => {
  it("rounds to the nearest bucket boundary", () => {
    expect(timeBucket(0, 40)).toBe(0);
    expect(timeBucket(19, 40)).toBe(0);
    expect(timeBucket(21, 40)).toBe(40);
    expect(timeBucket(1000, 40)).toBe(1000);
  });
});

describe("fxCacheKey", () => {
  it("joins every component into one string", () => {
    expect(fxCacheKey(40, "10-20", "off", "1-2-3", "ripple-1,2,3-0.5-true-false", "none"))
      .toBe("40_10-20_off_1-2-3_ripple-1,2,3-0.5-true-false_none");
  });

  it("changes when any single component changes", () => {
    const base = fxCacheKey(40, "10-20", "off", "", "fx", "none");
    expect(fxCacheKey(80, "10-20", "off", "", "fx", "none")).not.toBe(base);
    expect(fxCacheKey(40, "11-20", "off", "", "fx", "none")).not.toBe(base);
    expect(fxCacheKey(40, "10-20", "on", "", "fx", "none")).not.toBe(base);
  });
});

describe("isStaleFxResponse", () => {
  it("is not stale when the wanted key is unchanged since the request was issued", () => {
    expect(isStaleFxResponse("40_a", "40_a")).toBe(false);
  });

  it("is stale once the wanted key has moved on", () => {
    expect(isStaleFxResponse("40_a", "80_b")).toBe(true);
  });
});
