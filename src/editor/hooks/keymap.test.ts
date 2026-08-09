import { describe, expect, it } from "vitest";
import { keyAction } from "./keymap";

describe("keyAction", () => {
  it("is guarded against Ctrl - Ctrl+Z must not also fire add-zoom", () => {
    expect(keyAction({ key: "z", ctrlKey: true, metaKey: false, altKey: false, repeat: false }, false)).toBeNull();
  });

  it("is guarded against Cmd (metaKey) the same way", () => {
    expect(keyAction({ key: "z", ctrlKey: false, metaKey: true, altKey: false, repeat: false }, false)).toBeNull();
  });

  it("is guarded against Alt", () => {
    expect(keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: true, repeat: false }, false)).toBeNull();
  });

  it("ignores a held-down repeat for zoom", () => {
    expect(keyAction({ key: "z", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false)).toBeNull();
  });

  it("ignores a held-down repeat for spotlight", () => {
    expect(keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false)).toBeNull();
  });

  it("maps ? (Shift+/) to the shortcuts overlay", () => {
    expect(keyAction({ key: "?", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("overlay");
  });

  it("ignores a held-down repeat for the shortcuts overlay (holding ? must not flicker it)", () => {
    expect(keyAction({ key: "?", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false)).toBeNull();
  });

  it("maps a plain z to zoom", () => {
    expect(keyAction({ key: "z", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("zoom");
  });

  it("maps a plain s to spotlight", () => {
    expect(keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("spotlight");
  });

  it("maps Delete to delete only when something is selected", () => {
    expect(keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBeNull();
    expect(keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, true)).toBe("delete");
  });

  it("maps Backspace the same as Delete", () => {
    expect(keyAction({ key: "Backspace", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, true)).toBe("delete");
  });

  it("allows delete to repeat (holding it is fine, unlike zoom/spotlight)", () => {
    expect(keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, true)).toBe("delete");
  });

  it("maps space to play", () => {
    expect(keyAction({ key: " ", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("play");
  });

  it("allows play to repeat", () => {
    expect(keyAction({ key: " ", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false)).toBe("play");
  });

  it("returns null for an unmapped key", () => {
    expect(keyAction({ key: "q", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBeNull();
  });

  it("is case-insensitive for z/s", () => {
    expect(keyAction({ key: "Z", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("zoom");
    expect(keyAction({ key: "S", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe("spotlight");
  });
});
