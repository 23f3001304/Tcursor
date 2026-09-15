// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import { coalesce } from "./useRafCoalesced";

function fakeScheduler() {
  let nextId = 1;
  const queued = new Map<number, () => void>();
  const schedule = (cb: () => void) => {
    const id = nextId++;
    queued.set(id, cb);
    return id;
  };
  const cancelSchedule = (id: number) => {
    queued.delete(id);
  };
  const runFrame = () => {
    const cbs = [...queued.values()];
    queued.clear();
    cbs.forEach((cb) => cb());
  };
  return { schedule, cancelSchedule, runFrame, pendingCount: () => queued.size };
}

describe("coalesce", () => {
  it("does not call onCommit before a frame runs", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(1);
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("calls onCommit once per frame, with the LATEST value", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule, runFrame } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(1);
    c(2);
    c(3);
    runFrame();
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(3);
  });

  it("schedules a new frame after each one runs", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule, runFrame } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(1);
    runFrame();
    c(2);
    runFrame();
    expect(onCommit).toHaveBeenCalledTimes(2);
    expect(onCommit.mock.calls).toEqual([[1], [2]]);
  });

  it("a call while a frame is already queued does not schedule a second one", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule, pendingCount } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(1);
    expect(pendingCount()).toBe(1);
    c(2);
    expect(pendingCount()).toBe(1);
  });

  it("flush() applies a pending value immediately and cancels the queued frame", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule, runFrame, pendingCount } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(5);
    c.flush();
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(5);
    expect(pendingCount()).toBe(0);
    runFrame();
    expect(onCommit).toHaveBeenCalledTimes(1);
  });

  it("flush() is a no-op with nothing pending", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    expect(() => c.flush()).not.toThrow();
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("cancel() drops a pending value without applying it", () => {
    const onCommit = vi.fn();
    const { schedule, cancelSchedule, runFrame } = fakeScheduler();
    const c = coalesce(onCommit, schedule, cancelSchedule);
    c(1);
    c.cancel();
    runFrame();
    expect(onCommit).not.toHaveBeenCalled();
  });
});
