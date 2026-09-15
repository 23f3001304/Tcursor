import { describe, expect, it } from "vitest";
import { canonicalKeys, evalKeys, keysToString, MAX_KEYS, parseKeys, toKeys } from "./keys";
import { ease } from "../timeline/model/layoutTrack";
import { CURVES, key, parsed } from "./keysFixture";

describe("parseKeys", () => {
  it("accepts every canonical curve and round-trips it byte for byte", () => {
    for (const c of CURVES) expect(keysToString(parsed(c.wire))).toBe(c.canon);
    for (const c of CURVES) expect(keysToString(parsed(c.canon))).toBe(c.canon);
  });

  it("takes any run of spaces inside a key", () => {
    expect(keysToString(parsed("keys(0  0 0 0   0.333 0 b,1 1 -0.333 0 0 0 b)"))).toBe(CURVES[4].canon);
  });

  it("rejects everything off-form", () => {
    const nine = Array.from({ length: 9 }, (_, i) => `${i / 8} ${i / 8} 0 0 0 0 b`).join(",");
    expect(parseKeys(`keys(${nine})`)).toBeNull();
    expect(nine.split(",").length).toBe(MAX_KEYS + 1);
    expect(parseKeys("keys(0.1 0 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 b,0.9 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 x,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 b,0.5 1 0 0 0 0 b,0.4 1 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 b,0.5 1 0 0 0 0 b,0.5 1 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 x 0 0 0 0 b,1 1 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 b)")).toBeNull();
    expect(parseKeys("keys(garbage)")).toBeNull();
    expect(parseKeys("keys()")).toBeNull();
    expect(parseKeys("smooth")).toBeNull();
    expect(parseKeys("spring(140,7,1)")).toBeNull();
    expect(parseKeys("keys(0 0 0 0 0 0 b,1 1 0 0 0 0 b) trailing")).toBeNull();
  });
});

describe("canonicalKeys", () => {
  it("sorts by t", () => {
    const out = canonicalKeys({ keys: [key(1, 1), key(0.4, 0.5), key(0, 0)] });
    expect(out.keys.map((k) => k.t)).toEqual([0, 0.4, 1]);
  });

  it("clamps a handle that reaches past its neighbour, and only in x", () => {
    const out = canonicalKeys({ keys: [key(0, 0, [0.9, 2]), key(0.4, 0.5), key(1, 1, [0, 0], [-3, -2])] });
    expect(out.keys[0].out).toEqual([0.4, 2]);
    expect(out.keys[2].in).toEqual([-0.6, -2]);
    expect(canonicalKeys({ keys: [key(0, 0, [-0.5, 0]), key(1, 1)] }).keys[0].out[0]).toBe(0);
  });

  it("zeroes the outer handles and rounds to 3 decimals, with no negative zero", () => {
    const out = canonicalKeys({
      keys: [key(0, 0, [0.1234, 0.5], [-0.9, -0.9]), key(1, 1, [-0.4, 0.2], [-0.2, -0.0001])],
    });
    expect(out.keys[0].in).toEqual([0, 0]);
    expect(out.keys[1].out).toEqual([0, 0]);
    expect(out.keys[0].out).toEqual([0.123, 0.5]);
    expect(keysToString(out)).toBe(
      "keys(0.000 0.000 0.000 0.000 0.123 0.500 b,1.000 1.000 -0.200 0.000 0.000 0.000 b)",
    );
  });
});

describe("toKeys", () => {
  const closeToEase = (name: string) => {
    const k = toKeys(name);
    expect(k).not.toBeNull();
    for (let i = 0; i <= 100; i++) expect(evalKeys(k!, i / 100)).toBeCloseTo(ease(name, i / 100), 3);
  };

  it("converts smooth and a cubic to an equivalent bezier, to 1e-3", () => {
    expect(keysToString(toKeys("smooth")!)).toBe(CURVES[4].canon);
    closeToEase("smooth");
    closeToEase("cubic(0.25,0.100,0.25,1.000)");
    closeToEase("cubic(0.420,0.000,0.580,1.000)");
    expect(keysToString(toKeys("cubic(0.250,0.100,0.250,1.000)")!)).toBe(
      "keys(0.000 0.000 0.000 0.000 0.250 0.100 b,1.000 1.000 -0.750 0.000 0.000 0.000 b)",
    );
  });

  it("converts the named curves", () => {
    expect(keysToString(toKeys("linear")!)).toBe(
      "keys(0.000 0.000 0.000 0.000 0.000 0.000 l,1.000 1.000 0.000 0.000 0.000 0.000 l)",
    );
    closeToEase("linear");
    closeToEase("ease_in");
    closeToEase("ease_out");
    const eio = toKeys("ease_in_out")!;
    const worst = Math.max(
      ...Array.from({ length: 101 }, (_, i) =>
        Math.abs(evalKeys(eio, i / 100) - ease("ease_in_out", i / 100)),
      ),
    );
    expect(worst).toBeLessThan(0.006);
  });

  it("passes a keys string straight through, canonicalised", () => {
    expect(keysToString(toKeys(CURVES[0].wire)!)).toBe(CURVES[0].canon);
  });

  it("is null for a spring and for anything ease does not know", () => {
    expect(toKeys("spring")).toBeNull();
    expect(toKeys("spring(140,7,1)")).toBeNull();
    expect(toKeys("keys(garbage)")).toBeNull();
    expect(toKeys("wobble")).toBeNull();
    expect(toKeys("")).toBeNull();
  });
});
