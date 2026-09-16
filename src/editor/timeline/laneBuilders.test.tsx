import { describe, it, expect } from "vitest";
import { fxLabel } from "./laneBuilders";

describe("fxLabel", () => {
  it("names the kind, so one lane can carry four things", () => {
    const text = (kind: string) => JSON.stringify(fxLabel({ kind } as never));
    expect(text("spotlight")).toContain("Spotlight");
    expect(text("blur")).toContain("Blur");
    expect(text("pixelate")).toContain("Pixelate");
    expect(text("highlight")).toContain("Highlight");
  });

  it("falls back to the spotlight look for a kind it does not know", () => {
    expect(JSON.stringify(fxLabel({ kind: "nonsense" } as never))).toContain("Spotlight");
  });
});
