import { describe, expect, it } from "vitest";
import { newSilences, silenceToast } from "./useSilences";

describe("newSilences", () => {
  it("drops a span an existing cut already covers and keeps the rest", () => {
    const cuts = [{ start_ms: 1000, end_ms: 3000 }];
    expect(newSilences([[1200, 2000], [4000, 5000], [2500, 3500]], cuts)).toEqual([[4000, 5000], [2500, 3500]]);
  });
  it("keeps everything with no cuts", () => {
    expect(newSilences([[0, 100]], [])).toEqual([[0, 100]]);
  });
});

describe("silenceToast", () => {
  it("counts the cuts and sums the time they take out", () => {
    expect(silenceToast([[0, 1000], [2000, 3400]])).toBe("Removed 2 silences, 2.4 s");
    expect(silenceToast([[0, 700]])).toBe("Removed 1 silence, 0.7 s");
  });
  it("says so when nothing was found", () => {
    expect(silenceToast([])).toBe("No silences found");
  });
});
