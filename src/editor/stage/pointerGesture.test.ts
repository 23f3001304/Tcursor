import { describe, it, expect, vi } from "vitest";
import { attachPointerGesture, type GestureTarget } from "./pointerGesture";

/** A fake window-like target recording registered listeners per event type, so tests can dispatch
 *  a named event without a real DOM and assert exactly what's (still) registered afterward. */
function fakeTarget(): GestureTarget & { dispatch(type: string): void; count(type: string): number } {
  const listeners = new Map<string, Set<(e: PointerEvent) => void>>();
  return {
    addEventListener(type, cb) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type)!.add(cb);
    },
    removeEventListener(type, cb) {
      listeners.get(type)?.delete(cb);
    },
    dispatch(type) {
      for (const cb of listeners.get(type) ?? []) cb({} as PointerEvent);
    },
    count(type) {
      return listeners.get(type)?.size ?? 0;
    },
  };
}

describe("attachPointerGesture", () => {
  it("attaches pointermove, pointerup, and pointercancel on call", () => {
    const target = fakeTarget();
    attachPointerGesture(vi.fn(), vi.fn(), target);
    expect(target.count("pointermove")).toBe(1);
    expect(target.count("pointerup")).toBe(1);
    expect(target.count("pointercancel")).toBe(1);
  });

  it("calls onMove for every pointermove dispatch", () => {
    const target = fakeTarget();
    const onMove = vi.fn();
    attachPointerGesture(onMove, vi.fn(), target);
    target.dispatch("pointermove");
    target.dispatch("pointermove");
    expect(onMove).toHaveBeenCalledTimes(2);
  });

  it("pointerup calls onEnd exactly once and removes all three listeners", () => {
    const target = fakeTarget();
    const onEnd = vi.fn();
    attachPointerGesture(vi.fn(), onEnd, target);
    target.dispatch("pointerup");
    expect(onEnd).toHaveBeenCalledTimes(1);
    expect(target.count("pointermove")).toBe(0);
    expect(target.count("pointerup")).toBe(0);
    expect(target.count("pointercancel")).toBe(0);
  });

  it("pointercancel (no pointerup at all) ALSO calls onEnd and removes all three listeners - the exact fix", () => {
    // Reproduces the round-2 bug: only pointerup was wired before, so a cancelled sequence left
    // the gesture "stuck active" (its caller's drag flag never reset) and leaked both listeners.
    const target = fakeTarget();
    const onEnd = vi.fn();
    attachPointerGesture(vi.fn(), onEnd, target);
    target.dispatch("pointercancel");
    expect(onEnd).toHaveBeenCalledTimes(1);
    expect(target.count("pointermove")).toBe(0);
    expect(target.count("pointerup")).toBe(0);
    expect(target.count("pointercancel")).toBe(0);
  });

  it("onMove no longer fires after the gesture ends via pointercancel", () => {
    const target = fakeTarget();
    const onMove = vi.fn();
    attachPointerGesture(onMove, vi.fn(), target);
    target.dispatch("pointermove");
    target.dispatch("pointercancel");
    target.dispatch("pointermove"); // should be a no-op now
    expect(onMove).toHaveBeenCalledTimes(1);
  });

  it("does not error if pointerup and pointercancel somehow both fire (idempotent cleanup)", () => {
    const target = fakeTarget();
    const onEnd = vi.fn();
    attachPointerGesture(vi.fn(), onEnd, target);
    target.dispatch("pointerup");
    expect(() => target.dispatch("pointercancel")).not.toThrow();
    // the second dispatch finds nothing registered (already removed), so onEnd isn't called again
    expect(onEnd).toHaveBeenCalledTimes(1);
  });

  it("returns a detach function that removes all three listeners WITHOUT calling onEnd - the unmount safety net", () => {
    const target = fakeTarget();
    const onEnd = vi.fn();
    const detach = attachPointerGesture(vi.fn(), onEnd, target);
    detach();
    expect(onEnd).not.toHaveBeenCalled();
    expect(target.count("pointermove")).toBe(0);
    expect(target.count("pointerup")).toBe(0);
    expect(target.count("pointercancel")).toBe(0);
  });

  it("onMove no longer fires after detach()", () => {
    const target = fakeTarget();
    const onMove = vi.fn();
    const detach = attachPointerGesture(onMove, vi.fn(), target);
    detach();
    target.dispatch("pointermove");
    expect(onMove).not.toHaveBeenCalled();
  });

  it("detach() after a real end (pointerup) is a harmless no-op - does not call onEnd again", () => {
    const target = fakeTarget();
    const onEnd = vi.fn();
    const detach = attachPointerGesture(vi.fn(), onEnd, target);
    target.dispatch("pointerup");
    expect(() => detach()).not.toThrow();
    expect(onEnd).toHaveBeenCalledTimes(1);
  });
});
