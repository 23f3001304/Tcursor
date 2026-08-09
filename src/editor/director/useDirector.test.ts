import { describe, expect, it, vi } from "vitest";
import { createQueue } from "../hooks/opQueue";
import { fetchStepsToReveal, planOrCancel } from "./useDirector";

// `planOrCancel` is the seam `useDirector.run` uses to make the `ai_plan` fetch window
// cancellable: Esc/scrim/Stop-pill only ever touch `cancelRef.current` (they can't abort the
// in-flight HTTP call), so the fetch always resolves - what changes is whether `run` is allowed
// to react to it. These tests exercise that seam directly, without mounting the hook.
describe("planOrCancel", () => {
  it("resolves to the fetched plan when cancel was never requested", async () => {
    const cancelRef = { current: false };
    await expect(planOrCancel(cancelRef, async () => ["step-a", "step-b"])).resolves.toEqual(["step-a", "step-b"]);
  });

  it("resolves to null - never surfacing the plan - when cancel lands while the fetch is still in flight", async () => {
    const cancelRef = { current: false };
    const fetchPlan = () => new Promise<string[]>((resolve) => {
      cancelRef.current = true; // simulates Esc/scrim landing mid-fetch, before it settles
      setTimeout(() => resolve(["step-a"]), 5);
    });
    await expect(planOrCancel(cancelRef, fetchPlan)).resolves.toBeNull();
  });

  it("still resolves to the plan when cancel fires AFTER the fetch already settled (too late to matter)", async () => {
    const cancelRef = { current: false };
    const steps = await planOrCancel(cancelRef, async () => ["step-a"]);
    cancelRef.current = true; // a click landing after the await returned shouldn't rewrite history
    expect(steps).toEqual(["step-a"]);
  });

  // Runs `planOrCancel` through the exact `createQueue` seam `Editor.tsx` serializes the whole AI
  // pass through, proving a planning-phase cancel both (a) applies zero steps and (b) releases the
  // queue immediately - a fn enqueued after the cancelled pass isn't left waiting behind it.
  it("releases the opQueue and applies nothing when the pass is cancelled during planning", async () => {
    const enqueue = createQueue();
    const cancelRef = { current: false };
    const applied: string[] = [];

    const pass = () => enqueue(async () => {
      const steps = await planOrCancel(cancelRef, () => new Promise<string[]>((res) => setTimeout(() => res(["a", "b"]), 5)));
      if (steps === null) return "Stopped before planning finished";
      applied.push(...steps);
      return "done";
    });

    const result = pass();
    cancelRef.current = true; // Esc fires while the fetch above is still pending
    await expect(result).resolves.toBe("Stopped before planning finished");
    expect(applied).toEqual([]);
    await expect(enqueue(async () => "next")).resolves.toBe("next"); // queue wasn't left blocked
  });
});

// `fetchStepsToReveal` is the seam that fixes the no-op-undo-entry bug: `run` used to call
// `record(doc0)` unconditionally, BEFORE the cancellable fetch, so cancelling during a slow
// (first-run-model-load) planning fetch still left a spurious snapshot on the undo stack even
// though nothing was ever applied. `record` must fire only once a reveal is actually about to
// happen - never on a cancelled-during-planning pass.
describe("fetchStepsToReveal", () => {
  it("does NOT call record when the pass is cancelled during planning", async () => {
    const cancelRef = { current: false };
    const record = vi.fn();
    const fetchPlan = () => new Promise<string[]>((resolve) => {
      cancelRef.current = true; // Esc/scrim lands mid-fetch, same as the planOrCancel case above
      setTimeout(() => resolve(["step-a"]), 5);
    });
    await expect(fetchStepsToReveal(cancelRef, fetchPlan, record)).resolves.toBeNull();
    expect(record).not.toHaveBeenCalled();
  });

  it("calls record exactly once, after the fetch resolves and before returning the steps to reveal", async () => {
    const cancelRef = { current: false };
    const order: string[] = [];
    const record = vi.fn(() => { order.push("record"); });
    const fetchPlan = () => new Promise<string[]>((resolve) => {
      setTimeout(() => { order.push("fetch-resolved"); resolve(["step-a", "step-b"]); }, 5);
    });

    const steps = await fetchStepsToReveal(cancelRef, fetchPlan, record);
    order.push("reveal-would-start-here"); // stands in for `run`'s `reveal(...)` call, right after

    expect(steps).toEqual(["step-a", "step-b"]);
    expect(record).toHaveBeenCalledTimes(1);
    expect(order).toEqual(["fetch-resolved", "record", "reveal-would-start-here"]);
  });

  it("does not call record when the fetch rejects", async () => {
    const cancelRef = { current: false };
    const record = vi.fn();
    await expect(fetchStepsToReveal(cancelRef, () => Promise.reject(new Error("no models")), record))
      .rejects.toThrow("no models");
    expect(record).not.toHaveBeenCalled();
  });
});
