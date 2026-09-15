import { describe, expect, it } from "vitest";
import type { WhisperModelDto } from "../../../shared/ipc";
import { actionFor, languageOptions, sizeWarning } from "./modelCopy";

const dto = (over: Partial<WhisperModelDto> = {}): WhisperModelDto => ({
  id: "base.en",
  label: "Base (English)",
  bytes: 147_964_211,
  installed: false,
  multilingual: false,
  ...over,
});

describe("the transcribe card's copy", () => {
  it("states the download size in MB, rounded, before anything is downloaded", () => {
    expect(sizeWarning(147_964_211)).toBe("141 MB download, once");
    expect(sizeWarning(487_614_201)).toBe("465 MB download, once");
  });

  it("offers the download first and the transcribe only once the model is on disk", () => {
    expect(actionFor(dto(), "idle")).toEqual({ kind: "download", label: "Download model", disabled: false });
    expect(actionFor(dto({ installed: true }), "idle")).toEqual({
      kind: "transcribe",
      label: "Transcribe audio",
      disabled: false,
    });
  });

  it("says what it is doing while it works and never offers a second run", () => {
    expect(actionFor(dto({ installed: true }), "decoding").disabled).toBe(true);
    expect(actionFor(dto({ installed: true }), "transcribing").label).toBe("Transcribing");
    expect(actionFor(dto(), "downloading").label).toBe("Downloading");
  });

  it("asks for a model when none is picked", () => {
    expect(actionFor(null, "idle")).toEqual({ kind: "pick", label: "Pick a model", disabled: true });
  });

  it("uses no em dashes anywhere", () => {
    const all = [sizeWarning(1), actionFor(dto(), "idle").label, actionFor(null, "idle").label];
    const EM_DASH = String.fromCharCode(0x2014);
    for (const s of all) expect(s).not.toContain(EM_DASH);
  });

  it("keeps a busy phase busy even when nothing is picked, so the bar never loses its caption", () => {
    expect(actionFor(null, "transcribing")).toEqual({ kind: "busy", label: "Transcribing", disabled: true });
  });

  it("names the language options and refuses Auto on an English-only model", () => {
    expect(languageOptions(dto()).find((o) => o.value === "auto")?.disabled).toBe(true);
    expect(languageOptions(dto({ multilingual: true })).find((o) => o.value === "auto")?.disabled).toBe(
      false,
    );
    expect(languageOptions(null).map((o) => o.value)).toEqual(["en", "auto"]);
  });
});
