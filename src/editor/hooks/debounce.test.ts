import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { debounce } from "./debounce";

beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); });

describe("debounce", () => {
  it("does not call fn before `wait` elapses", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d(1);
    vi.advanceTimersByTime(79);
    expect(fn).not.toHaveBeenCalled();
  });

  it("calls fn once, with the LATEST args, `wait` ms after the last call", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d(1);
    vi.advanceTimersByTime(40);
    d(2);
    vi.advanceTimersByTime(40); // 80ms since d(1), but only 40ms since d(2) - should NOT fire yet
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(40); // now 80ms since d(2)
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith(2);
  });

  it("collapses a burst of rapid calls into exactly one trailing call", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    for (let i = 0; i < 20; i++) { d(i); vi.advanceTimersByTime(5); } // 20 calls, 5ms apart - always within the window
    vi.advanceTimersByTime(80);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith(19);
  });

  it("cancel() drops a pending call", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d(1);
    d.cancel();
    vi.advanceTimersByTime(200);
    expect(fn).not.toHaveBeenCalled();
  });

  it("cancel() is a no-op with nothing pending", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    expect(() => d.cancel()).not.toThrow();
  });

  it("flush() runs a pending call immediately, synchronously", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d(1);
    d.flush();
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith(1);
    // the trailing timer must not ALSO fire later
    vi.advanceTimersByTime(200);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("flush() is a no-op with nothing pending", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d.flush();
    expect(fn).not.toHaveBeenCalled();
  });

  it("a call after flush() schedules a fresh trailing call", () => {
    const fn = vi.fn();
    const d = debounce(fn, 80);
    d(1);
    d.flush();
    d(2);
    vi.advanceTimersByTime(80);
    expect(fn).toHaveBeenCalledTimes(2);
    expect(fn).toHaveBeenLastCalledWith(2);
  });
});
