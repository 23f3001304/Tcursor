import { describe, it, expect } from "vitest";
import { formatTimer } from "./formatTimer";

describe("formatTimer", () => {
  it("formats milliseconds as M:SS", () => {
    expect(formatTimer(0)).toBe("0:00");
    expect(formatTimer(5_000)).toBe("0:05");
    expect(formatTimer(65_000)).toBe("1:05");
    expect(formatTimer(600_000)).toBe("10:00");
  });
});
