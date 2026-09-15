import { describe, expect, it } from "vitest";
import { friendlyAiError } from "./friendlyAiError";

describe("friendlyAiError", () => {
  it("maps the no-models-installed failure (pick_model's exact wording)", () => {
    expect(
      friendlyAiError("No Ollama models are installed. Pull one first, e.g.: ollama pull llama3.2"),
    ).toEqual({ title: "No local model installed", hint: "Run: ollama pull llama3.2" });
  });

  it("maps the unreachable-Ollama failure (ollama::chat's exact wording)", () => {
    expect(
      friendlyAiError("Could not reach Ollama at localhost:11434. Is it running? Try: ollama serve"),
    ).toEqual({ title: "Ollama isn't running", hint: "Start the Ollama app, then try again" });
  });

  it("maps a generic connection-refused/timeout message the same way", () => {
    expect(friendlyAiError("Error: connect ECONNREFUSED 127.0.0.1:11434").title).toBe("Ollama isn't running");
    expect(friendlyAiError("request timed out").title).toBe("Ollama isn't running");
  });

  it("passes any other error through unchanged, with no hint", () => {
    expect(friendlyAiError("parse error: unexpected token")).toEqual({
      title: "parse error: unexpected token",
      hint: null,
    });
  });
});
