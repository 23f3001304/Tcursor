import { describe, expect, it } from "vitest";
import { truncateToastText } from "./toastText";

describe("truncateToastText", () => {
  it("passes short text through unchanged", () => {
    expect(truncateToastText("Undid")).toBe("Undid");
  });

  it("passes text exactly at the cap through unchanged", () => {
    const text = "a".repeat(120);
    expect(truncateToastText(text)).toBe(text);
  });

  it("truncates a long raw warning on a word boundary with an ellipsis", () => {
    const text =
      "webcam decode failed after 143 frames - the camera panel is frozen from that point on: decoder returned an unexpected end of stream while reading packet 88213";
    const out = truncateToastText(text);
    expect(out.length).toBeLessThanOrEqual(120);
    expect(out.endsWith("…")).toBe(true);
    expect(out.endsWith(" …")).toBe(false);
  });

  it("respects a custom max", () => {
    expect(truncateToastText("hello world", 8)).toBe("hello…");
  });
});
