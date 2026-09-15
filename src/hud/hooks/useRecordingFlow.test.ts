import { describe, it, expect } from "vitest";
import { settleStop } from "./useRecordingFlow";

const fulfilled = <T>(value: T): PromiseFulfilledResult<T> => ({ status: "fulfilled", value });
const rejected = (reason: unknown): PromiseRejectedResult => ({ status: "rejected", reason });

describe("settleStop", () => {
  it("a take whose screen stop returned zero frames is an error, never a folder to open", () => {
    const out = settleStop(fulfilled({ folder: "C:/rec-0", frames: 0 }), fulfilled(undefined));
    expect("err" in out && /No video was captured/.test(out.err)).toBe(true);
  });

  it("returns the folder with no camWarn when both the screen and webcam stops settle fine", () => {
    const out = settleStop(fulfilled({ folder: "C:/rec-1", frames: 100 }), fulfilled(undefined));
    expect(out).toEqual({ folder: "C:/rec-1", camWarn: undefined });
  });

  it(
    "returns the folder PLUS a camWarn when only the webcam side rejected (fix round 1, item 2 - " +
      "this used to be a bare console.warn, invisible in the running app)",
    () => {
      const out = settleStop(
        fulfilled({ folder: "C:/rec-1", frames: 100 }),
        rejected(new Error("InvalidStateError")),
      );
      expect("folder" in out).toBe(true);
      if ("folder" in out) {
        expect(out.folder).toBe("C:/rec-1");
        expect(out.camWarn).toMatch(/webcam/i);
      }
    },
  );

  it(
    "fails the whole stop when the screen side rejected, even though the webcam side succeeded " +
      "(the H1 regression: Promise.all used to fail this way for the OPPOSITE, wrong reason)",
    () => {
      const out = settleStop(rejected("disk full"), fulfilled(undefined));
      expect(out).toEqual({ err: "Recording failed: disk full" });
    },
  );

  it("still fails via the screen error when BOTH rejected - camWarn is dropped, not merged in", () => {
    const out = settleStop(rejected("disk full"), rejected(new Error("cam gone")));
    expect(out).toEqual({ err: "Recording failed: disk full" });
    expect("camWarn" in out).toBe(false);
  });
});
