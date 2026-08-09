import { describe, it, expect } from "vitest";
import { resetCameraAppearance, DEFAULT_APPEARANCE } from "./appearanceFields";
import type { AppearanceSettings } from "../settings/settings";

// A settings object where every mode (including screen) has been dragged away from its
// factory default, so a reset that touches the wrong scope is easy to catch.
const dirty: AppearanceSettings = {
  screen: {
    pad: 0.05, screen_size: 0.8, screen_radius: 0.03,
    cam_size: 0.5, cam_shape: "rect", cam_radius: 0.2, cam_corner: "top_right",
    cam_margin_x: 0.05, cam_margin_y: 0.05, cam_aspect: "wide", cam_ring: { width: 0.1, color: [1, 2, 3] },
  },
  screen_only: { ...DEFAULT_APPEARANCE.screen_only, pad: 0.09 },
  camera: { ...DEFAULT_APPEARANCE.camera, cam_size: 0.5 },
  camera_only: { ...DEFAULT_APPEARANCE.camera_only, cam_shape: "circle" },
  presenter: { ...DEFAULT_APPEARANCE.presenter, screen_radius: 0.04 },
};

describe("resetCameraAppearance (Task 26 commit 1 - CameraPanel reset scope bug)", () => {
  it("resets only the screen mode's camera-owned knobs, to the exact Rust defaults", () => {
    const out = resetCameraAppearance(dirty);
    expect(out.screen.cam_size).toBe(DEFAULT_APPEARANCE.screen.cam_size);
    expect(out.screen.cam_margin_x).toBe(DEFAULT_APPEARANCE.screen.cam_margin_x);
    expect(out.screen.cam_margin_y).toBe(DEFAULT_APPEARANCE.screen.cam_margin_y);
    expect(out.screen.cam_shape).toBe(DEFAULT_APPEARANCE.screen.cam_shape);
    expect(out.screen.cam_radius).toBe(DEFAULT_APPEARANCE.screen.cam_radius);
    expect(out.screen.cam_aspect).toBe(DEFAULT_APPEARANCE.screen.cam_aspect);
    expect(out.screen.cam_corner).toBe(DEFAULT_APPEARANCE.screen.cam_corner);
    expect(out.screen.cam_ring).toBe(DEFAULT_APPEARANCE.screen.cam_ring);
  });

  it("leaves pad/screen_size/screen_radius untouched (BackgroundPanel owns pad/screen_radius)", () => {
    const out = resetCameraAppearance(dirty);
    expect(out.screen.pad).toBe(0.05);
    expect(out.screen.screen_size).toBe(0.8);
    expect(out.screen.screen_radius).toBe(0.03);
  });

  it("leaves the other four appearance modes completely untouched", () => {
    const out = resetCameraAppearance(dirty);
    expect(out.screen_only).toBe(dirty.screen_only);
    expect(out.camera).toBe(dirty.camera);
    expect(out.camera_only).toBe(dirty.camera_only);
    expect(out.presenter).toBe(dirty.presenter);
  });
});
