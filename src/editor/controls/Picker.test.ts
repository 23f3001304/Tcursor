import { describe, it, expect } from "vitest";
import { pickerNextIndex } from "./Picker";

describe("pickerNextIndex (Task 26 commit 2 - Picker keyboard a11y)", () => {
  it("ArrowDown from nothing-active starts at the first option", () => {
    expect(pickerNextIndex("ArrowDown", -1, 3)).toBe(0);
  });

  it("ArrowUp from nothing-active starts at the last option", () => {
    expect(pickerNextIndex("ArrowUp", -1, 3)).toBe(2);
  });

  it("ArrowDown/ArrowUp step by one and clamp at the ends", () => {
    expect(pickerNextIndex("ArrowDown", 1, 3)).toBe(2);
    expect(pickerNextIndex("ArrowDown", 2, 3)).toBe(2);
    expect(pickerNextIndex("ArrowUp", 1, 3)).toBe(0);
    expect(pickerNextIndex("ArrowUp", 0, 3)).toBe(0);
  });

  it("returns null for keys it doesn't handle", () => {
    expect(pickerNextIndex("Enter", 0, 3)).toBeNull();
    expect(pickerNextIndex("Tab", 0, 3)).toBeNull();
  });

  it("returns null for an empty option list", () => {
    expect(pickerNextIndex("ArrowDown", -1, 0)).toBeNull();
  });
});
