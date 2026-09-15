// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { webcamStopAction, raceStopOrTimeout, useWebcamRecorder } from "./useWebcamRecorder";
import { appendWebcam } from "../../shared/ipc";

vi.mock("../../shared/ipc", () => ({ appendWebcam: vi.fn(() => Promise.resolve()) }));

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
      const never = new Promise<void>(() => {});
      const pending = raceStopOrTimeout(never, 4000);
      await vi.advanceTimersByTimeAsync(4000);
      await expect(pending).resolves.toBe("timeout");
    } finally {
      vi.useRealTimers();
    }
  });
});

class FakeRecorder {
  static last: FakeRecorder | null = null;
  state: RecordingState = "inactive";
  ondataavailable: ((e: { data: { size: number; arrayBuffer: () => Promise<ArrayBuffer> } }) => void) | null =
    null;
  onstop: (() => void) | null = null;
  constructor(public stream: MediaStream) {
    FakeRecorder.last = this;
  }
  start() {
    this.state = "recording";
  }
  stop() {
    this.state = "inactive";
    this.onstop?.();
  }
  chunk(bytes: number[]) {
    this.ondataavailable?.({
      data: { size: bytes.length, arrayBuffer: () => Promise.resolve(new Uint8Array(bytes).buffer) },
    });
  }
}

describe("useWebcamRecorder segments", () => {
  let host: HTMLDivElement;
  let root: Root;
  let api: ReturnType<typeof useWebcamRecorder>;
  const Probe = () => {
    api = useWebcamRecorder();
    return null;
  };
  const stream = { id: "s1" } as MediaStream;

  beforeEach(() => {
    vi.stubGlobal("MediaRecorder", FakeRecorder);
    host = document.createElement("div");
    document.body.appendChild(host);
    root = createRoot(host);
    act(() => root.render(createElement(Probe)));
  });
  afterEach(() => {
    act(() => root.unmount());
    host.remove();
    vi.clearAllMocks();
  });

  const feed = async (bytes: number[]) => {
    FakeRecorder.last!.chunk(bytes);
    await act(async () => {
      await api.stop();
    });
  };

  it("streams the take's first segment to the plain webcam file", async () => {
    act(() => api.start(stream, "C:/rec-1"));
    expect(api.folder()).toBe("C:/rec-1");
    expect(api.segment()).toBe(1);
    await feed([1, 2, 3]);
    expect(appendWebcam).toHaveBeenCalledWith("C:/rec-1", new Uint8Array([1, 2, 3]), 1);
  });

  it("carries the segment index of the recorder that produced the chunk", async () => {
    act(() => api.start(stream, "C:/rec-1", 2));
    expect(api.segment()).toBe(2);
    await feed([9]);
    expect(appendWebcam).toHaveBeenCalledWith("C:/rec-1", new Uint8Array([9]), 2);
  });

  it("a second switch moves on again, and no chunk is written under the old index", async () => {
    act(() => api.start(stream, "C:/rec-1", 2));
    await feed([1]);
    act(() => api.start(stream, "C:/rec-1", 3));
    await feed([2]);
    expect(api.segment()).toBe(3);
    expect(vi.mocked(appendWebcam).mock.calls.map((c) => c[2])).toEqual([2, 3]);
  });

  it("an empty chunk is not written at all", async () => {
    act(() => api.start(stream, "C:/rec-1", 2));
    await feed([]);
    expect(appendWebcam).not.toHaveBeenCalled();
  });
});
