import { describe, it, expect } from "vitest";
import { exactKey, wantsExact, SETTLE_MS, type ExactFrame } from "./useExactFrame";

const held = (key: string): ExactFrame => ({ key, img: {} as HTMLImageElement });

describe("exactKey", () => {
  it("files a frame under its rounded instant and its edit generation", () => {
    expect(exactKey(1234.6, 3)).toBe("1235|3");
    expect(exactKey(-5, 1)).toBe("0|1");
  });

  it("changes with either the instant or the doc, so a stale frame never matches", () => {
    expect(exactKey(1000, 1)).not.toBe(exactKey(1001, 1));
    expect(exactKey(1000, 1)).not.toBe(exactKey(1000, 2));
  });
});

describe("wantsExact", () => {
  it("asks only when paused, with a folder, with no draft, and not already held", () => {
    expect(wantsExact(false, false, "C:/rec", "10|1", null)).toBe(true);
    expect(wantsExact(true, false, "C:/rec", "10|1", null)).toBe(false);
    expect(wantsExact(false, true, "C:/rec", "10|1", null)).toBe(false);
    expect(wantsExact(false, false, "", "10|1", null)).toBe(false);
    expect(wantsExact(false, false, "C:/rec", "10|1", held("10|1"))).toBe(false);
    expect(wantsExact(false, false, "C:/rec", "10|2", held("10|1"))).toBe(true);
  });

  it("settles for longer than a frame and shorter than a beat", () => {
    expect(SETTLE_MS).toBeGreaterThan(16);
    expect(SETTLE_MS).toBeLessThan(400);
  });
});
