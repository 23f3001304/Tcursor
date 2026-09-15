import { describe, expect, it } from "vitest";
import { engineDisplayName } from "./engineName";

describe("engineDisplayName", () => {
  it("shortens the HF GGUF proxy id from the audit screenshot", () => {
    expect(engineDisplayName("hf.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF:Q8_0")).toBe(
      "Qwythos 9B (Q8)",
    );
  });

  it("finds size/quant in the tag half of a plain Ollama id, not just the name half", () => {
    expect(engineDisplayName("llama3.1:8b-instruct-q4_0")).toBe("llama3.1 8B (Q4)");
  });

  it("falls back to just the family name when there's no size or quant to find", () => {
    expect(engineDisplayName("mistral")).toBe("mistral");
  });

  it("strips a host/org prefix even with no quant tag", () => {
    expect(engineDisplayName("hf.co/some-org/Phi-3-Mini")).toBe("Phi-3-Mini");
  });

  it("passes through an empty/whitespace id without throwing", () => {
    expect(engineDisplayName("")).toBe("");
    expect(engineDisplayName("   ")).toBe("");
  });

  it("is idempotent-ish on an already-short id", () => {
    expect(engineDisplayName("qwen2.5:14b")).toBe("qwen2.5 14B");
  });
});
