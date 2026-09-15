import { describe, it, expect, vi } from "vitest";
import { pollFor, CAMERA_WAIT_MS } from "./useSourceSwitch";

describe("pollFor", () => {
  it("returns the first value the predicate accepts, without waiting", async () => {
    await expect(
      pollFor(
        () => "live",
        (v) => v === "live",
        CAMERA_WAIT_MS,
      ),
    ).resolves.toBe("live");
  });

  it("keeps looking while the value is still the old one", async () => {
    vi.useFakeTimers();
    try {
      const seen = ["old", "old", "new"];
      const pending = pollFor(
        () => seen.shift() ?? "new",
        (v) => v === "new",
        1000,
        100,
      );
      await vi.advanceTimersByTimeAsync(1000);
      await expect(pending).resolves.toBe("new");
    } finally {
      vi.useRealTimers();
    }
  });

  it("gives up with null once the budget is spent (a camera that never starts)", async () => {
    vi.useFakeTimers();
    try {
      const pending = pollFor<string | null>(
        () => null,
        (v) => v != null,
        300,
        100,
      );
      await vi.advanceTimersByTimeAsync(400);
      await expect(pending).resolves.toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });
});
