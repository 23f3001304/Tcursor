import { describe, it, expect } from "vitest";
import type { LayoutSeg } from "../../../shared/edit";
import type { AppearanceSettings, LayoutPreset } from "../../../hud/settings/settings";
import { DEFAULT_APPEARANCE } from "../../../hud/preferences/appearanceFields";
import {
  BUILTIN_PRESET,
  addPreset,
  layoutAtPlayhead,
  nextPresetId,
  presetNameError,
  removePreset,
  renamePreset,
  resetLayout,
} from "./layoutPresets";

const seg = (id: string, start_ms: number, end_ms: number, layout: string): LayoutSeg => ({
  id,
  start_ms,
  end_ms,
  layout,
  transition_ms: 300,
  easing: "smooth",
  transition_out_ms: 0,
  easing_out: "smooth",
});

const look = (name: string): AppearanceSettings => ({
  ...DEFAULT_APPEARANCE,
  presenter: { ...DEFAULT_APPEARANCE.presenter, pad: name.length / 100 },
});

const settingsWith = (layout_presets: LayoutPreset[]) => ({ ai_model: "x", layout_presets });

describe("layoutAtPlayhead", () => {
  const segs = [seg("l1", 0, 1000, "presenter"), seg("l2", 2000, 3000, "camera_only")];

  it("picks the layout of the segment the playhead is inside", () => {
    expect(layoutAtPlayhead(segs, 500)).toBe("presenter");
    expect(layoutAtPlayhead(segs, 2500)).toBe("camera_only");
  });

  it("falls back to screen in a gap, before the first segment, and with no segments at all", () => {
    expect(layoutAtPlayhead(segs, 1500)).toBe("screen");
    expect(layoutAtPlayhead(segs, 0 - 1)).toBe("screen");
    expect(layoutAtPlayhead([], 500)).toBe("screen");
  });

  it("treats a segment as half-open, so its own end belongs to whatever follows", () => {
    expect(layoutAtPlayhead(segs, 999)).toBe("presenter");
    expect(layoutAtPlayhead(segs, 1000)).toBe("screen");
  });

  it("ignores a layout name this build does not know, and lets the last overlapper win", () => {
    expect(layoutAtPlayhead([seg("l1", 0, 1000, "hologram")], 500)).toBe("screen");
    expect(layoutAtPlayhead([seg("a", 0, 1000, "camera"), seg("b", 400, 1000, "presenter")], 500)).toBe(
      "presenter",
    );
  });
});

describe("presetNameError", () => {
  const list: LayoutPreset[] = [{ id: "lp1", name: "Bold", appearance: DEFAULT_APPEARANCE }];

  it("refuses an empty or whitespace-only name", () => {
    expect(presetNameError("", list)).toBe("Give this look a name.");
    expect(presetNameError("   ", list)).toBe("Give this look a name.");
  });

  it("refuses a duplicate, case- and space-insensitively, and the built-in row's name too", () => {
    expect(presetNameError(" bold ", list)).toContain("already a look called");
    expect(presetNameError("Default", list)).toContain("already a look called");
  });

  it("refuses an over-long name", () => {
    expect(presetNameError("x".repeat(41), list)).toContain("under 40 characters");
  });

  it("allows a fresh name, and allows renaming a row to its own current name", () => {
    expect(presetNameError("Quiet", list)).toBeNull();
    expect(presetNameError("Bold", list, "lp1")).toBeNull();
  });
});

describe("the four list edits over a Settings-shaped value", () => {
  it("saves a look under the next free id, keeping every other settings field", () => {
    const out = addPreset(settingsWith([]), "  Bold  ", look("bold"));
    expect(out.ai_model).toBe("x");
    expect(out.layout_presets).toEqual([{ id: "lp1", name: "Bold", appearance: look("bold") }]);
    expect(nextPresetId(out.layout_presets)).toBe("lp2");
  });

  it("refuses to save a name that would not validate, rather than writing a duplicate", () => {
    const before = settingsWith([{ id: "lp1", name: "Bold", appearance: DEFAULT_APPEARANCE }]);
    expect(addPreset(before, "bold", DEFAULT_APPEARANCE)).toBe(before);
    expect(addPreset(before, "  ", DEFAULT_APPEARANCE)).toBe(before);
  });

  it("renames in place - same id, same position, same appearance", () => {
    const before = settingsWith([
      { id: "lp1", name: "Bold", appearance: look("a") },
      { id: "lp2", name: "Quiet", appearance: look("bb") },
    ]);
    const out = renamePreset(before, "lp1", "Loud");
    expect(out.layout_presets.map((p) => [p.id, p.name])).toEqual([
      ["lp1", "Loud"],
      ["lp2", "Quiet"],
    ]);
    expect(out.layout_presets[0].appearance).toEqual(look("a"));
    expect(renamePreset(before, "lp1", "Quiet")).toBe(before);
    expect(renamePreset(before, "nope", "Fine")).toBe(before);
  });

  it("deletes by id and cannot reach the built-in row, which is not in the list", () => {
    const before = settingsWith([{ id: "lp1", name: "Bold", appearance: DEFAULT_APPEARANCE }]);
    expect(removePreset(before, "lp1").layout_presets).toEqual([]);
    expect(removePreset(before, BUILTIN_PRESET.id).layout_presets).toHaveLength(1);
    expect(BUILTIN_PRESET.appearance).toEqual(DEFAULT_APPEARANCE);
  });
});

describe("resetLayout", () => {
  it("restores ONE layout and leaves the other four exactly as they were", () => {
    const dirty: AppearanceSettings = {
      ...DEFAULT_APPEARANCE,
      screen: { ...DEFAULT_APPEARANCE.screen, cam_size: 0.5, pad: 0.07 },
      presenter: { ...DEFAULT_APPEARANCE.presenter, screen_radius: 0.04 },
    };
    const out = resetLayout(dirty, "screen");
    expect(out.screen).toEqual(DEFAULT_APPEARANCE.screen);
    expect(out.presenter).toBe(dirty.presenter);
    expect(out.camera).toBe(dirty.camera);
    expect(out.camera_only).toBe(dirty.camera_only);
    expect(out.screen_only).toBe(dirty.screen_only);
  });
});
