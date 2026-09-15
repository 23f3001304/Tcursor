import { describe, expect, it } from "vitest";
import { createQueue } from "../util/opQueue";
import { planOrCancel } from "./useAiRun";

describe("planOrCancel", () => {
  it("resolves to the fetched run when cancel was never requested", async () => {
    const cancelRef = { current: false };
    await expect(planOrCancel(cancelRef, async () => ["p0", "p1"])).resolves.toEqual(["p0", "p1"]);
  });

  it("resolves to null, never surfacing the run, when cancel lands while the fetch is still in flight", async () => {
    const cancelRef = { current: false };
    const fetch = () =>
      new Promise<string[]>((resolve) => {
        cancelRef.current = true;
        setTimeout(() => resolve(["p0"]), 5);
      });
    await expect(planOrCancel(cancelRef, fetch)).resolves.toBeNull();
  });

  it("still resolves to the run when cancel fires after the fetch already settled", async () => {
    const cancelRef = { current: false };
    const got = await planOrCancel(cancelRef, async () => ["p0"]);
    cancelRef.current = true;
    expect(got).toEqual(["p0"]);
  });

  it("propagates a rejected fetch as-is", async () => {
    await expect(
      planOrCancel({ current: false }, () => Promise.reject(new Error("no models"))),
    ).rejects.toThrow("no models");
  });

  it("the op queue runs tasks in order and releases after each one", async () => {
    const enqueue = createQueue();
    const order: string[] = [];
    const slow = enqueue(async () => {
      await new Promise((r) => setTimeout(r, 5));
      order.push("apply");
      return 2;
    });
    const next = enqueue(async () => {
      order.push("undo");
      return "undo";
    });
    await expect(slow).resolves.toBe(2);
    await expect(next).resolves.toBe("undo");
    expect(order).toEqual(["apply", "undo"]);
  });
});
