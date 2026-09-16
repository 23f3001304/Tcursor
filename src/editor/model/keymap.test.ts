import { describe, expect, it } from "vitest";
import { isTypingTarget, keyAction, ownsSpace, resolveKeyAction, type TargetLike } from "./keymap";

const target = (over: Partial<TargetLike> = {}): TargetLike => ({
  tagName: "DIV",
  role: null,
  isContentEditable: false,
  ...over,
});
const key = (
  over: Partial<{ key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean; repeat: boolean }> = {},
) => ({ key: "z", ctrlKey: false, metaKey: false, altKey: false, repeat: false, ...over });
const ctx = (
  over: Partial<{ hasSel: boolean; modalOpen: boolean; shortcutsOpen: boolean; target: TargetLike }> = {},
) => ({ hasSel: false, modalOpen: false, shortcutsOpen: false, target: target(), ...over });

describe("keyAction", () => {
  it("is guarded against Ctrl - Ctrl+Z must not also fire add-zoom", () => {
    expect(
      keyAction({ key: "z", ctrlKey: true, metaKey: false, altKey: false, repeat: false }, false),
    ).toBeNull();
  });

  it("is guarded against Cmd (metaKey) the same way", () => {
    expect(
      keyAction({ key: "z", ctrlKey: false, metaKey: true, altKey: false, repeat: false }, false),
    ).toBeNull();
  });

  it("is guarded against Alt", () => {
    expect(
      keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: true, repeat: false }, false),
    ).toBeNull();
  });

  it("ignores a held-down repeat for zoom", () => {
    expect(
      keyAction({ key: "z", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false),
    ).toBeNull();
  });

  it("ignores a held-down repeat for spotlight", () => {
    expect(
      keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false),
    ).toBeNull();
  });

  it("maps ? (Shift+/) to the shortcuts overlay", () => {
    expect(keyAction({ key: "?", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "overlay",
    );
  });

  it("ignores a held-down repeat for the shortcuts overlay (holding ? must not flicker it)", () => {
    expect(
      keyAction({ key: "?", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false),
    ).toBeNull();
  });

  it("maps a plain z to zoom", () => {
    expect(keyAction({ key: "z", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "zoom",
    );
  });

  it("maps a plain s to spotlight", () => {
    expect(keyAction({ key: "s", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "spotlight",
    );
  });

  it("maps Delete to delete only when something is selected", () => {
    expect(
      keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false),
    ).toBeNull();
    expect(
      keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, true),
    ).toBe("delete");
  });

  it("maps Backspace the same as Delete", () => {
    expect(
      keyAction({ key: "Backspace", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, true),
    ).toBe("delete");
  });

  it("allows delete to repeat (holding it is fine, unlike zoom/spotlight)", () => {
    expect(
      keyAction({ key: "Delete", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, true),
    ).toBe("delete");
  });

  it("maps space to play", () => {
    expect(keyAction({ key: " ", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "play",
    );
  });

  it("allows play to repeat", () => {
    expect(keyAction({ key: " ", ctrlKey: false, metaKey: false, altKey: false, repeat: true }, false)).toBe(
      "play",
    );
  });

  it("returns null for an unmapped key", () => {
    expect(
      keyAction({ key: "q", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false),
    ).toBeNull();
  });

  it("is case-insensitive for z/s/t, and t adds a text item unless it is a repeat", () => {
    expect(keyAction({ key: "Z", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "zoom",
    );
    expect(keyAction({ key: "S", ctrlKey: false, metaKey: false, altKey: false, repeat: false }, false)).toBe(
      "spotlight",
    );
    expect(keyAction(key({ key: "t" }), false)).toBe("text");
    expect(keyAction(key({ key: "T" }), false)).toBe("text");
    expect(keyAction(key({ key: "t", repeat: true }), false)).toBeNull();
  });
});

describe("isTypingTarget", () => {
  it("is true for INPUT/TEXTAREA and contenteditable", () => {
    expect(isTypingTarget(target({ tagName: "INPUT" }))).toBe(true);
    expect(isTypingTarget(target({ tagName: "TEXTAREA" }))).toBe(true);
    expect(isTypingTarget(target({ isContentEditable: true }))).toBe(true);
  });

  it("is false for a plain element, or a button", () => {
    expect(isTypingTarget(target())).toBe(false);
    expect(isTypingTarget(target({ tagName: "BUTTON" }))).toBe(false);
  });
});

describe("ownsSpace", () => {
  it("is true for a native button/select - Space is their own default activation", () => {
    expect(ownsSpace(target({ tagName: "BUTTON" }))).toBe(true);
    expect(ownsSpace(target({ tagName: "SELECT" }))).toBe(true);
  });

  it("is true for a custom widget's ARIA role that owns Space (e.g. Switch, role=switch)", () => {
    expect(ownsSpace(target({ tagName: "DIV", role: "switch" }))).toBe(true);
  });

  it("is false for role=slider - a slider doesn't own Space (fixes the dead-key regression)", () => {
    expect(ownsSpace(target({ tagName: "DIV", role: "slider" }))).toBe(false);
  });

  it("is false for a plain, non-interactive element", () => {
    expect(ownsSpace(target())).toBe(false);
  });
});

describe("resolveKeyAction (the full decision useEditorKeymap acts on)", () => {
  it("is inert for every action while a modal is open (M4 scenario C)", () => {
    expect(resolveKeyAction(key({ key: "z" }), ctx({ modalOpen: true }))).toBeNull();
    expect(resolveKeyAction(key({ key: "s" }), ctx({ modalOpen: true }))).toBeNull();
    expect(resolveKeyAction(key({ key: " " }), ctx({ modalOpen: true }))).toBeNull();
    expect(resolveKeyAction(key({ key: "Delete" }), ctx({ hasSel: true, modalOpen: true }))).toBeNull();
    expect(resolveKeyAction(key({ key: "?" }), ctx({ modalOpen: true }))).toBeNull();
  });

  it("is inert while typing, same as before - modalOpen false", () => {
    expect(resolveKeyAction(key({ key: "s" }), ctx({ target: target({ tagName: "INPUT" }) }))).toBeNull();
  });

  it("blocks Space (only) when the target owns its own Space activation (M4 scenarios A/B)", () => {
    expect(resolveKeyAction(key({ key: " " }), ctx({ target: target({ tagName: "BUTTON" }) }))).toBeNull();
  });

  it("still fires Space when the target is a plain, non-interactive element", () => {
    expect(resolveKeyAction(key({ key: " " }), ctx())).toBe("play");
  });

  it("still fires Space (toggles play) when the target is a role=slider Slider", () => {
    expect(resolveKeyAction(key({ key: " " }), ctx({ target: target({ role: "slider" }) }))).toBe("play");
  });

  it("does NOT block z/s/delete/overlay for a target that owns Space - only Space itself is gated", () => {
    const c = ctx({ hasSel: true, target: target({ tagName: "BUTTON" }) });
    expect(resolveKeyAction(key({ key: "z" }), c)).toBe("zoom");
    expect(resolveKeyAction(key({ key: "s" }), c)).toBe("spotlight");
    expect(resolveKeyAction(key({ key: "Delete" }), c)).toBe("delete");
    expect(resolveKeyAction(key({ key: "?" }), c)).toBe("overlay");
  });

  it("passes through to keyAction's normal behavior with no modal and a non-owning target", () => {
    expect(resolveKeyAction(key({ key: "z" }), ctx())).toBe("zoom");
    expect(resolveKeyAction(key({ key: "q" }), ctx())).toBeNull();
  });

  describe("? special-cased ahead of the modal bail", () => {
    it("opens normally with nothing open", () => {
      expect(resolveKeyAction(key({ key: "?" }), ctx())).toBe("overlay");
    });

    it("still fires (to close it) while ShortcutsOverlay is the open modal", () => {
      expect(resolveKeyAction(key({ key: "?" }), ctx({ modalOpen: true, shortcutsOpen: true }))).toBe(
        "overlay",
      );
    });

    it("stays blocked behind a DIFFERENT modal - must not pop the overlay on top of it", () => {
      expect(resolveKeyAction(key({ key: "?" }), ctx({ modalOpen: true, shortcutsOpen: false }))).toBeNull();
    });

    it("is still blocked by typing and still respects its own repeat guard", () => {
      expect(
        resolveKeyAction(
          key({ key: "?" }),
          ctx({ shortcutsOpen: true, target: target({ tagName: "INPUT" }) }),
        ),
      ).toBeNull();
      expect(resolveKeyAction(key({ key: "?", repeat: true }), ctx({ shortcutsOpen: true }))).toBeNull();
    });
  });
});
