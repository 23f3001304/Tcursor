import { describe, it, expect } from "vitest";
import { shouldClearOverride } from "./overrideClear";

describe("shouldClearOverride", () => {
  it("never clears while the gesture is still active, no matter what changed", () => {
    expect(shouldClearOverride(true, true, 5, 3)).toBe(false);
    expect(shouldClearOverride(true, true, 3, 3)).toBe(false);
  });

  it("never clears when no override is set", () => {
    expect(shouldClearOverride(false, false, 5, 3)).toBe(false);
  });

  it("clears once the source value differs from the gesture-start snapshot", () => {
    expect(shouldClearOverride(false, true, 7, 3)).toBe(true);
  });

  it("does not clear while the source value still equals its gesture-start snapshot (commit still pending)", () => {
    expect(shouldClearOverride(false, true, 3, 3)).toBe(false);
  });

  it("clears on an undo/Reset that lands mid-wait, even though it never matches the override's own value", () => {
    const dragValue = 9;
    const settledAtGestureStart = 3;
    const valueAfterExternalReset = 0;
    expect(
      shouldClearOverride(false, dragValue !== null, valueAfterExternalReset, settledAtGestureStart),
    ).toBe(true);
  });

  it("a rejected/clamped commit that leaves the source value completely unchanged never resolves on its own", () => {
    expect(shouldClearOverride(false, true, 3, 3)).toBe(false);
  });

  describe("with a custom eq (structural types, e.g. Stage's aim point)", () => {
    const aimEq = (a: [number, number] | null, b: [number, number] | null) =>
      (a === null && b === null) ||
      (!!a && !!b && Math.abs(a[0] - b[0]) < 1e-9 && Math.abs(a[1] - b[1]) < 1e-9);

    it("does not clear while the aim point still matches its pre-drag snapshot", () => {
      expect(shouldClearOverride(false, true, [0.5, 0.5] as [number, number], [0.5, 0.5], aimEq)).toBe(false);
    });

    it("clears when the aim point moves to a different value", () => {
      expect(shouldClearOverride(false, true, [0.6, 0.5] as [number, number], [0.5, 0.5], aimEq)).toBe(true);
    });

    it("clears when the aim point becomes null (deselected) - the exact ghost-reticle bug this fixes", () => {
      expect(shouldClearOverride(false, true, null, [0.5, 0.5] as [number, number] | null, aimEq)).toBe(true);
    });

    it("does not clear when both the pre-drag snapshot and the current point are null", () => {
      expect(shouldClearOverride(false, true, null, null, aimEq)).toBe(false);
    });
  });
});
