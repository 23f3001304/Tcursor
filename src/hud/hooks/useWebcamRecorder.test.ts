import { describe, it, expect, vi } from "vitest";
import { webcamStopAction, raceStopOrTimeout } from "./useWebcamRecorder";

describe("webcamStopAction", () => {
  it("skips when there is no recorder at all", () => {
    expect(webcamStopAction(null)).toBe("skip");
  });

  it("skips when the browser already auto-stopped it (H1 - a second mr.stop() would throw)", () => {
    expect(webcamStopAction("inactive")).toBe("skip");
  });

  it("attempts a stop while actively recording", () => {
    expect(webcamStopAction("recording")).toBe("stop");
  });

  it("attempts a stop while paused", () => {
    expect(webcamStopAction("paused")).toBe("stop");
  });
});

describe("raceStopOrTimeout", () => {
  it('resolves "stopped" when onDone settles before the timeout', async () => {
    await expect(raceStopOrTimeout(Promise.resolve(), 4000)).resolves.toBe("stopped");
  });

  it('resolves "timeout" when onDone never settles (H1\'s secondary hazard)', async () => {
    vi.useFakeTimers();
    try {
      const never = new Promise<void>(() => {}); // an onstop that never fires
      const pending = raceStopOrTimeout(never, 4000);
      await vi.advanceTimersByTimeAsync(4000);
      await expect(pending).resolves.toBe("timeout");
    } finally {
      vi.useRealTimers();
    }
  });
});
