import { describe, expect, it } from "vitest";
import { keyAction, resolveKeyAction } from "./keymap";

const key = (k: string, over: Partial<Parameters<typeof keyAction>[0]> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  repeat: false,
  ...over,
});
const target = { tagName: "DIV", role: null, isContentEditable: false };

describe("the split key", () => {
  it("is B, and S is still the spotlight", () => {
    expect(keyAction(key("b"), false)).toBe("split");
    expect(keyAction(key("B"), false)).toBe("split");
    expect(keyAction(key("s"), false)).toBe("spotlight");
  });

  it("does not repeat while the key is held", () => {
    expect(keyAction(key("b", { repeat: true }), false)).toBeNull();
  });

  it("is off under a modifier and inside a text field", () => {
    expect(keyAction(key("b", { ctrlKey: true }), false)).toBeNull();
    expect(
      resolveKeyAction(key("b"), {
        hasSel: false,
        modalOpen: false,
        shortcutsOpen: false,
        target: { tagName: "INPUT", role: null, isContentEditable: false },
      }),
    ).toBeNull();
  });

  it("is off while a modal is open", () => {
    expect(
      resolveKeyAction(key("b"), { hasSel: false, modalOpen: true, shortcutsOpen: false, target }),
    ).toBeNull();
  });
});
