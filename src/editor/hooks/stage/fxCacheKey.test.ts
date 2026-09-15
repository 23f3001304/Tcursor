import { describe, expect, it } from "vitest";
import { timeBucket, fxCacheKey, isStaleFxResponse, fxResponseAction, spotParamsKey } from "./fxCacheKey";

const spot = {
  dim: 0.6,
  radius: 0.13,
  feather: 0.1,
  mode: "classic",
  tint: [130, 90, 255] as [number, number, number],
};

describe("spotParamsKey", () => {
  it("is 'off' when no spotlight resolves", () => {
    expect(spotParamsKey(null, 0.8, true)).toBe("off");
  });

  it("changes when any part of the spotlight's LOOK changes", () => {
    const base = spotParamsKey(spot, 0.8, true);
    expect(spotParamsKey({ ...spot, dim: 0.7 }, 0.8, true)).not.toBe(base);
    expect(spotParamsKey({ ...spot, radius: 0.2 }, 0.8, true)).not.toBe(base);
    expect(spotParamsKey({ ...spot, feather: 0.2 }, 0.8, true)).not.toBe(base);
    expect(spotParamsKey({ ...spot, mode: "halo" }, 0.8, true)).not.toBe(base);
    expect(spotParamsKey({ ...spot, tint: [0, 0, 0] }, 0.8, true)).not.toBe(base);
    expect(spotParamsKey(spot, 0.5, true)).not.toBe(base);
  });

  it("changes when the alpha path flips, so a cached image is never blitted on the wrong basis", () => {
    expect(spotParamsKey(spot, 0.8, true)).not.toBe(spotParamsKey(spot, 0.8, false));
  });

  it("ignores alpha entirely - that is what keeps the key stable across a fade", () => {
    expect(spotParamsKey({ ...spot }, 0.8, true)).toBe(spotParamsKey({ ...spot }, 0.8, true));
    expect(Object.keys(spot)).not.toContain("alpha");
  });
});

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
    expect(fxCacheKey(40, "10-20", "off", "1-2-3", "ripple-1,2,3-0.5-true-false", "none")).toBe(
      "40_10-20_off_1-2-3_ripple-1,2,3-0.5-true-false_none",
    );
  });

  it("changes when any single component changes", () => {
    const base = fxCacheKey(40, "10-20", "off", "", "fx", "none");
    expect(fxCacheKey(80, "10-20", "off", "", "fx", "none")).not.toBe(base);
    expect(fxCacheKey(40, "11-20", "off", "", "fx", "none")).not.toBe(base);
    expect(fxCacheKey(40, "10-20", "on", "", "fx", "none")).not.toBe(base);
    expect(fxCacheKey(40, "10-20", "off", "1-2-3", "fx", "none")).not.toBe(base);
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

describe("fxResponseAction", () => {
  it("drops a stale response instead of applying it", () => {
    expect(fxResponseAction("40_a", "80_b", "data:image/png;base64,x")).toEqual({ kind: "stale" });
  });

  it("applies a null response (nothing to draw) exactly like a real one - both latch", () => {
    const nullResult = fxResponseAction("40_a", "40_a", null);
    const urlResult = fxResponseAction("40_a", "40_a", "data:image/png;base64,x");
    expect(nullResult).toEqual({ kind: "apply", imageUrl: null });
    expect(urlResult).toEqual({ kind: "apply", imageUrl: "data:image/png;base64,x" });
    expect(nullResult.kind).toBe(urlResult.kind);
  });

  it("is not stale when the wanted key is unchanged, even with no request in flight yet", () => {
    expect(
      fxResponseAction("0_none_off__off-1,2,3-0.5-none", "0_none_off__off-1,2,3-0.5-none", null),
    ).toEqual({ kind: "apply", imageUrl: null });
  });
});
