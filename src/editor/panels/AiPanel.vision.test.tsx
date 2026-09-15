// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";

vi.mock("../../shared/ipc", () => ({
  listOllamaModels: () =>
    Promise.resolve([
      { name: "llama3.2", vision: false },
      { name: "qwen2.5vl:7b", vision: true },
    ]),
}));

const { AiPanel } = await import("./AiPanel");

const props = {
  running: false,
  exporting: false,
  error: null,
  progress: null,
  onRun: () => {},
  onChangeModel: () => {},
  onAutoModel: () => {},
  onClose: () => {},
  run: null,
  skipped: new Set<string>(),
  applying: false,
  previewId: null,
  onToggleItem: () => {},
  onPreviewItem: () => {},
  onApply: () => {},
  onDiscard: () => {},
};

let root: Root, container: HTMLDivElement;
const badges = () => Array.from(container.querySelectorAll(".e-picker-badge")).map((e) => e.textContent);

const show = async (model: string) => {
  await act(async () => {
    root.render(<AiPanel {...props} model={model} />);
  });
};

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(async () => {
  await act(async () => {
    root.unmount();
  });
  container.remove();
});

describe("engine picker", () => {
  it("badges a vision-capable engine on the picker's own button", async () => {
    await show("qwen2.5vl:7b");
    expect(container.querySelector('[aria-label="Engine"]')).toBeTruthy();
    expect(badges()).toEqual(["Vision"]);
  });

  it("leaves a text-only engine bare, so the badge is a claim and not decoration", async () => {
    await show("llama3.2");
    expect(container.querySelector('[aria-label="Engine"]')).toBeTruthy();
    expect(badges()).toEqual([]);
  });
});
