// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";

const emitters: Record<string, (payload: unknown) => void> = {};
vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: (e: { payload: unknown }) => void) => {
    emitters[name] = (payload) => cb({ payload });
    return Promise.resolve(() => {
      delete emitters[name];
    });
  },
}));
const MODEL = { id: "base.en", label: "Base (English)", bytes: 1, installed: true, multilingual: false };
const transcribeProject = vi.fn((_folder: string) => Promise.resolve());
const downloadWhisperModel = vi.fn((_id: string) => Promise.resolve());
const whisperModels = vi.fn(() => Promise.resolve([MODEL]));
vi.mock("../../../shared/ipc", () => ({
  whisperModels: () => whisperModels(),
  downloadWhisperModel: (id: string) => downloadWhisperModel(id),
  transcribeProject: (folder: string) => transcribeProject(folder),
}));

import { useTranscribe } from "./useTranscribe";

let api: ReturnType<typeof useTranscribe>;
let done = 0;
let root: Root;

function Harness() {
  api = useTranscribe("C:/rec", () => {
    done += 1;
  });
  return null;
}
const emit = (name: string, payload: unknown) =>
  act(() => {
    emitters[name](payload);
  });

beforeEach(async () => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  done = 0;
  transcribeProject.mockClear();
  downloadWhisperModel.mockClear();
  whisperModels.mockClear();
  root = createRoot(document.createElement("div"));
  await act(async () => {
    root.render(<Harness />);
  });
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
});

describe("useTranscribe", () => {
  it("reads the model table on mount", () => {
    expect(api.models).toEqual([MODEL]);
    expect(api.phase).toBe("idle");
  });

  it("walks idle to decoding to transcribing and back to idle, reporting the percent", () => {
    act(() => {
      api.transcribe();
    });
    expect(transcribeProject).toHaveBeenCalledWith("C:/rec");
    expect(api.phase).toBe("decoding");
    emit("asr-progress", { phase: "decode", pct: 50 });
    expect(api.phase).toBe("decoding");
    expect(api.pct).toBe(50);
    emit("asr-progress", { phase: "transcribe", pct: 20 });
    expect(api.phase).toBe("transcribing");
    emit("asr-done", 7);
    expect(api.phase).toBe("idle");
    expect(api.error).toBeNull();
    expect(done).toBe(1);
  });

  it("surfaces a backend error verbatim and returns to idle so the button works again", () => {
    act(() => {
      api.transcribe();
    });
    emit("asr-error", "This recording has no audio to transcribe.");
    expect(api.phase).toBe("idle");
    expect(api.error).toBe("This recording has no audio to transcribe.");
    expect(done).toBe(0);
  });

  it("a download reports its own percent and re-reads the table when it lands", () => {
    act(() => {
      api.download("base.en");
    });
    expect(downloadWhisperModel).toHaveBeenCalledWith("base.en");
    expect(api.phase).toBe("downloading");
    emit("asr-download-progress", { id: "base.en", done: 25, total: 200 });
    expect(api.pct).toBe(13);
    const reads = whisperModels.mock.calls.length;
    emit("asr-download-done", "base.en");
    expect(api.phase).toBe("idle");
    expect(whisperModels.mock.calls.length).toBe(reads + 1);
  });

  it("clears the previous error when a new run starts", () => {
    act(() => {
      api.transcribe();
    });
    emit("asr-error", "nope");
    expect(api.error).toBe("nope");
    act(() => {
      api.transcribe();
    });
    expect(api.error).toBeNull();
  });
});
